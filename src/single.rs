use std::cmp;
use std::cell::RefCell;
use std::iter;
use std::mem;
use std::sync::Mutex;

use {MIN_CAPACITY, INITIAL_SIZE};


/// A simple arena allocator.
pub struct Arena<T> {
    chunks: RefCell<ChunkList<T>>,
}


impl<T> Arena<T> {
    /// Create a new `Arena` with a default size of approximately 1024 bytes.
    pub fn new() -> Arena<T> {
        Arena { chunks: RefCell::new(ChunkList::new()) }
    }

    /// Create a new `Arena` with enough capacity for at least `n` `T`s without
    /// a reallocation.
    pub fn with_capacity(n: usize) -> Arena<T> {
        Arena { chunks: RefCell::new(ChunkList::with_capacity(n)) }
    }

    /// Allocate a single object in the arena.
    pub fn alloc(&self, t: T) -> &mut T {
        unsafe {
            mem::transmute::<&mut T, &mut T>(&mut self.chunks
                                                      .borrow_mut()
                                                      .alloc_extend(iter::once(t))
                                                      [0])
        }
    }

    /// Allocate an arbitrary number of objects in the arena.
    pub fn alloc_extend<I: Iterator<Item = T>>(&self, iterable: I) -> &mut [T] {
        unsafe {
            mem::transmute::<&mut [T], &mut [T]>(self.chunks.borrow_mut().alloc_extend(iterable))
        }
    }
}


/// A thread-safe arena.
pub struct AtomicArena<T> {
    chunks: Mutex<ChunkList<T>>,
}


impl<T> AtomicArena<T> {
    /// Create a new `AtomicArena` with a default size of approximately 1024 bytes.
    pub fn new() -> AtomicArena<T> {
        AtomicArena { chunks: Mutex::new(ChunkList::new()) }
    }

    /// Create a new `AtomicArena` with enough capacity for at least `n` `T`s without
    /// a reallocation.
    pub fn with_capacity(n: usize) -> AtomicArena<T> {
        AtomicArena { chunks: Mutex::new(ChunkList::with_capacity(n)) }
    }

    /// Allocate a single object in the arena.
    pub fn alloc(&self, t: T) -> &mut T {
        unsafe {
            mem::transmute::<&mut T, &mut T>(&mut self.chunks
                                                      .lock()
                                                      .unwrap()
                                                      .alloc_extend(iter::once(t))
                                                      [0])
        }
    }

    /// Allocate an arbitrary number of objects in the arena.
    pub fn alloc_extend<I: Iterator<Item = T>>(&self, iterable: I) -> &mut [T] {
        unsafe {
            mem::transmute::<&mut [T], &mut [T]>(self.chunks.lock().unwrap().alloc_extend(iterable))
        }
    }
}


/// The list of chunks - `Vec<T>`s holding allocated objects - comprising the
/// arena.
pub struct ChunkList<T> {
    current: Vec<T>,
    rest: Vec<Vec<T>>,
}


impl<T> ChunkList<T> {
    /// Create a new `ChunkList` of approximately `INITIAL_SIZE` bytes with
    /// enough capacity for `INITIAL_SIZE / mem::size_of::<T>()` `T`s in the
    /// `current` chunk.
    fn new() -> ChunkList<T> {
        let size = cmp::max(1, mem::size_of::<T>());
        ChunkList::with_capacity(INITIAL_SIZE / size)
    }


    /// Create a new `ChunkList` with capacity for at least `n` `T`s in the
    /// `current` chunk.
    fn with_capacity(n: usize) -> ChunkList<T> {
        let n = cmp::max(MIN_CAPACITY, n);
        ChunkList {
            current: Vec::with_capacity(n),
            rest: vec![],
        }
    }


    /// Reserve a new `current` chunk with enough space for at least `additional`
    /// elements.
    #[inline(never)]
    #[cold]
    fn reserve(&mut self, additional: usize) {
        let double_cap = self.current
            .capacity()
            .checked_mul(2)
            .expect("capacity overflow");
        let required_cap = additional
            .checked_next_power_of_two()
            .expect("capacity overflow");
        let new_capacity = cmp::max(double_cap, required_cap);
        let chunk = mem::replace(&mut self.current, Vec::with_capacity(new_capacity));
        self.rest.push(chunk);
    }


    fn alloc_extend<I: IntoIterator<Item = T>>(&mut self, iterable: I) -> &mut [T] {
        let mut iter = iterable.into_iter();

        let iter_min_len = iter.size_hint().0;
        let mut next_item_index;

        if self.current.len() + iter_min_len > self.current.capacity() {
            self.reserve(iter_min_len);
            self.current.extend(iter);
            next_item_index = 0;
        } else {
            next_item_index = self.current.len();
            let mut i = 0;
            while let Some(elem) = iter.next() {
                if self.current.len() == self.current.capacity() {
                    self.reserve(i + 1);
                    let previous_chunk = self.rest.last_mut().unwrap();
                    let previous_chunk_len = previous_chunk.len();
                    self.current
                        .extend(previous_chunk.drain(previous_chunk_len - i..));
                    self.current.push(elem);
                    self.current.extend(iter);
                    next_item_index = 0;
                    break;
                } else {
                    self.current.push(elem);
                    i += 1;
                }
            }
        }

        &mut self.current[next_item_index..]
    }
}


#[cfg(test)]
mod test {
    use std::cell::Cell;
    use super::*;

    fn assert_send_and_sync<T: Send + Sync>(_: T) {}

    #[allow(dead_code)]
    fn assert_atomic_send_sync() {
        assert_send_and_sync(AtomicArena::<u8>::new());
    }


    struct DropTracker<'a>(&'a Cell<u32>);


    impl<'a> Drop for DropTracker<'a> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }


    struct Node<'a, 'b: 'a>(Option<&'a Node<'a, 'b>>, u32, DropTracker<'b>);


    #[test]
    fn arena_as_intended() {
        let drop_counter = Cell::new(0);
        {
            let arena = Arena::with_capacity(2);

            let mut node: &Node = arena.alloc(Node(None, 1, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.borrow().rest.len(), 0);

            node = arena.alloc(Node(Some(node), 2, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.borrow().rest.len(), 0);

            node = arena.alloc(Node(Some(node), 3, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.borrow().rest.len(), 1);

            node = arena.alloc(Node(Some(node), 4, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.borrow().rest.len(), 1);

            assert_eq!(node.1, 4);
            assert_eq!(node.0.unwrap().1, 3);
            assert_eq!(node.0.unwrap().0.unwrap().1, 2);
            assert_eq!(node.0.unwrap().0.unwrap().0.unwrap().1, 1);
            assert!(node.0.unwrap().0.unwrap().0.unwrap().0.is_none());

            mem::drop(node);
            assert_eq!(drop_counter.get(), 0);

            let mut node: &Node = arena.alloc(Node(None, 5, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.borrow().rest.len(), 1);

            node = arena.alloc(Node(Some(node), 6, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.borrow().rest.len(), 1);

            node = arena.alloc(Node(Some(node), 7, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.borrow().rest.len(), 2);

            assert_eq!(drop_counter.get(), 0);

            assert_eq!(node.1, 7);
            assert_eq!(node.0.unwrap().1, 6);
            assert_eq!(node.0.unwrap().0.unwrap().1, 5);
            assert!(node.0.unwrap().0.unwrap().0.is_none());

            assert_eq!(drop_counter.get(), 0);
        }
        assert_eq!(drop_counter.get(), 7);
    }


    #[test]
    fn atomic_arena_as_intended() {
        let drop_counter = Cell::new(0);
        {
            let arena = AtomicArena::with_capacity(2);

            let mut node: &Node = arena.alloc(Node(None, 1, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest.len(), 0);

            node = arena.alloc(Node(Some(node), 2, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest.len(), 0);

            node = arena.alloc(Node(Some(node), 3, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest.len(), 1);

            node = arena.alloc(Node(Some(node), 4, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest.len(), 1);

            assert_eq!(node.1, 4);
            assert_eq!(node.0.unwrap().1, 3);
            assert_eq!(node.0.unwrap().0.unwrap().1, 2);
            assert_eq!(node.0.unwrap().0.unwrap().0.unwrap().1, 1);
            assert!(node.0.unwrap().0.unwrap().0.unwrap().0.is_none());

            mem::drop(node);
            assert_eq!(drop_counter.get(), 0);

            let mut node: &Node = arena.alloc(Node(None, 5, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest.len(), 1);

            node = arena.alloc(Node(Some(node), 6, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest.len(), 1);

            node = arena.alloc(Node(Some(node), 7, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest.len(), 2);

            assert_eq!(drop_counter.get(), 0);

            assert_eq!(node.1, 7);
            assert_eq!(node.0.unwrap().1, 6);
            assert_eq!(node.0.unwrap().0.unwrap().1, 5);
            assert!(node.0.unwrap().0.unwrap().0.is_none());

            assert_eq!(drop_counter.get(), 0);
        }
        assert_eq!(drop_counter.get(), 7);
    }
}
