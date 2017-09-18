use std::iter;
use std::mem;
use std::sync::Mutex;

use chunk_list::ChunkList;


/// A `Sync` arena.
pub struct Arena<T> {
    chunks: Mutex<ChunkList<T>>,
}


impl<T> Arena<T> {
    /// Create a new `Arena` with a default size of approximately 1024 bytes.
    pub fn new() -> Arena<T> {
        Arena { chunks: Mutex::new(ChunkList::new()) }
    }

    /// Create a new `Arena` with enough capacity for at least `n` `T`s without
    /// a reallocation.
    pub fn with_capacity(n: usize) -> Arena<T> {
        Arena { chunks: Mutex::new(ChunkList::with_capacity(n)) }
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


#[cfg(test)]
mod test {
    use std::cell::Cell;
    use std::mem;
    use super::*;

    fn assert_send_and_sync<T: Send + Sync>() {}

    #[allow(dead_code)]
    fn assert_atomic_send_sync() {
        assert_send_and_sync::<Arena<u8>>();
    }


    struct DropTracker<'a>(&'a Cell<u32>);


    impl<'a> Drop for DropTracker<'a> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }


    struct Node<'a, 'b: 'a>(Option<&'a Node<'a, 'b>>, u32, DropTracker<'b>);


    #[test]
    fn as_intended() {
        let drop_counter = Cell::new(0);
        {
            let arena = Arena::with_capacity(2);

            let mut node: &Node = arena.alloc(Node(None, 1, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest().len(), 0);

            node = arena.alloc(Node(Some(node), 2, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest().len(), 0);

            node = arena.alloc(Node(Some(node), 3, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest().len(), 1);

            node = arena.alloc(Node(Some(node), 4, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest().len(), 1);

            assert_eq!(node.1, 4);
            assert_eq!(node.0.unwrap().1, 3);
            assert_eq!(node.0.unwrap().0.unwrap().1, 2);
            assert_eq!(node.0.unwrap().0.unwrap().0.unwrap().1, 1);
            assert!(node.0.unwrap().0.unwrap().0.unwrap().0.is_none());

            mem::drop(node);
            assert_eq!(drop_counter.get(), 0);

            let mut node: &Node = arena.alloc(Node(None, 5, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest().len(), 1);

            node = arena.alloc(Node(Some(node), 6, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest().len(), 1);

            node = arena.alloc(Node(Some(node), 7, DropTracker(&drop_counter)));
            assert_eq!(arena.chunks.lock().unwrap().rest().len(), 2);

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

