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
