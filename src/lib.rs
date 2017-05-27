//! This crate contains a typed arena based on [`rust-typed-arena`]
//! (https://github.com/SimonSapin/rust-typed-arena), which is itself based on
//! the `TypedArena` used in rustc. The main difference between this crate and
//! the `typed_arena` crate is that this crate also provides an allocator which
//! uses a `Mutex` internally instead of a `RefCell`; thus, the `AtomicArena`
//! type is thread-safe.


// The initial size, in bytes, of a newly minted arena without a specified
// capacity.
const INITIAL_SIZE: usize = 1024;

// The minimum allowed capacity of an arena.
const MIN_CAPACITY: usize = 1;


mod single;


pub use single::{Arena, AtomicArena};
