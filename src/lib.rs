//! This crate contains a typed arena based on [`rust-typed-arena`]
//! (https://github.com/SimonSapin/rust-typed-arena), which is itself based on
//! the `TypedArena` used in rustc. The main difference between this crate and
//! the `typed_arena` crate is that this crate also provides an allocator which
//! uses a `Mutex` internally instead of a `RefCell`; thus, the `sync::Arena`
//! type is thread-safe.


// The initial size, in bytes, of a newly minted arena without a specified
// capacity.
const INITIAL_SIZE: usize = 1024;

// The minimum allowed capacity of an arena.
const MIN_CAPACITY: usize = 1;


mod chunk_list;

pub mod sync;
pub mod unsync;


#[cfg(test)]
mod test {
    use std::cell::Cell;
    use super::*;

    fn assert_send_and_sync<T: Send + Sync>() {}

    #[allow(dead_code)]
    fn assert_atomic_send_sync() {
        assert_send_and_sync::<sync::Arena<u8>>();
    }


    struct DropTracker<'a>(&'a Cell<u32>);


    impl<'a> Drop for DropTracker<'a> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }


    struct Node<'a, 'b: 'a>(Option<&'a Node<'a, 'b>>, u32, DropTracker<'b>);


    #[test]
    fn unsync_arena_as_intended() {
        let drop_counter = Cell::new(0);
        {
            let arena = unsync::Arena::with_capacity(2);

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
    fn sync_arena_as_intended() {
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
