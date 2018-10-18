# hypercore
[![crates.io version][1]][2] [![build status][3]][4]
[![downloads][5]][6] [![docs.rs docs][7]][8]

WIP. Secure, distributed, append-only log structure. Adapted from
[mafintosh/hypercore](https://github.com/mafintosh/hypercore).

- [Documentation][8]
- [Crates.io][2]

## Usage
```rust
extern crate hypercore;

use hypercore::Feed;
use std::path::PathBuf;

let path = PathBuf::from("./my-first-dataset");
let mut feed = Feed::new(&path).unwrap();

feed.append(b"hello").unwrap();
feed.append(b"world").unwrap();

println!("{:?}", feed.get(0)); // prints "hello"
println!("{:?}", feed.get(1)); // prints "world"
```

## Installation
```sh
$ cargo add hypercore
```

## Safety
This crate uses ``#![deny(unsafe_code)]`` to ensure everything is implemented in
100% Safe Rust.

## Contributing
Want to join us? Check out our ["Contributing" guide][contributing] and take a
look at some of these issues:

- [Issues labeled "good first issue"][good-first-issue]
- [Issues labeled "help wanted"][help-wanted]

## References
- [github.com/mafintosh/hypercore](https://github.com/mafintosh/hypercore)

## License
[MIT](./LICENSE-MIT) OR [Apache-2.0](./LICENSE-APACHE)

[1]: https://img.shields.io/crates/v/hypercore.svg?style=flat-square
[2]: https://crates.io/crates/hypercore
[3]: https://img.shields.io/travis/datrs/hypercore.svg?style=flat-square
[4]: https://travis-ci.org/datrs/hypercore
[5]: https://img.shields.io/crates/d/hypercore/master.svg?style=flat-square
[6]: https://crates.io/crates/hypercore
[7]: https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square
[8]: https://docs.rs/hypercore

[releases]: https://github.com/datrs/hypercore/releases
[contributing]: https://github.com/datrs/hypercore/blob/master.github/CONTRIBUTING.md
[good-first-issue]: https://github.com/datrs/hypercore/labels/good%20first%20issue
[help-wanted]: https://github.com/datrs/hypercore/labels/help%20wanted
