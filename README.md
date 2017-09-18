[![Build Status](https://travis-ci.org/sdleffler/colosseum-rs.svg?branch=master)](https://travis-ci.org/sdleffler/colosseum-rs)
[![Docs Status](https://docs.rs/colosseum/badge.svg)](https://docs.rs/colosseum)
[![On crates.io](https://img.shields.io/crates/v/colosseum.svg)](https://crates.io/crates/colosseum)

# `colosseum`: A variety of arena allocators for Rust

At present, the `colosseum` crate provides the following arena allocators:
 * `unsync::Arena`: a simple arena allocator for a single type.
 * `sync::Arena`: a thread-safe version of `unsync::Arena`.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
