# hypercore
[![crates.io version][1]][2] [![build status][3]][4]
[![downloads][5]][6] [![docs.rs docs][7]][8]

WIP. Secure, distributed, append-only log structure. Adapted from
[mafintosh/hypercore](https://github.com/mafintosh/hypercore).

- [Documentation][8]
- [Crates.io][2]

## Usage
```rust,ignore
extern crate hypercore;

use hypercore::Feed;
use std::path::PathBuf;

let path = PathBuf::from("./my-first-dataset");
let mut feed = Feed::new(path).unwrap();

feed.append(b"hello").unwrap();
feed.append(b"world").unwrap();

println!("{:?}", feed.get(0)); // prints "hello"
println!("{:?}", feed.get(1)); // prints "world"
```

## Data Structures
- __feed:__ The main data structure in Hypercore. Append-only log that uses
  multiple data structures and algorithms to safely store data.
- __data:__ Data that's written to the feed by users.
- __keypair:__ An `Ed25519` key pair used to encrypt data with.
- __signature:__ A cryptorgraphic certificate of authenticity for a given piece
  of code.
- __tree:__ A binary tree mapped as a `flat-tree` to keep an index of the
  current data.
- __bitfield:__ ???

## Installation
```sh
$ cargo add hypercore
```

## License
[MIT](./LICENSE-MIT) OR [Apache-2.0](./LICENSE-APACHE)

[1]: https://img.shields.io/crates/v/hypercore.svg?style=flat-square
[2]: https://crates.io/crates/hypercore
[3]: https://img.shields.io/travis/datrs/hypercore.svg?style=flat-square
[4]: https://travis-ci.org/datrs/hypercore
[5]: https://img.shields.io/crates/d/hypercore.svg?style=flat-square
[6]: https://crates.io/crates/hypercore
[7]: https://docs.rs/hypercore/badge.svg
[8]: https://docs.rs/hypercore
