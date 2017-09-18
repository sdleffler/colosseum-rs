use std::cmp;
use std::mem;

use {MIN_CAPACITY, INITIAL_SIZE};


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
    pub fn new() -> ChunkList<T> {
        let size = cmp::max(1, mem::size_of::<T>());
        ChunkList::with_capacity(INITIAL_SIZE / size)
    }


    /// Create a new `ChunkList` with capacity for at least `n` `T`s in the
    /// `current` chunk.
    pub fn with_capacity(n: usize) -> ChunkList<T> {
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


    pub fn alloc_extend<I: IntoIterator<Item = T>>(&mut self, iterable: I) -> &mut [T] {
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
