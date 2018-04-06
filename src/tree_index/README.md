Stateful tree index. Or well, stateful flat-tree. It's what happens when you
combine a flat-tree with a sparse-bitfield - which ends up being pretty cool!

Adapted from
[mafintosh/hypercore/lib/tree-index.js](https://github.com/mafintosh/hypercore/blob/master/lib/tree-index.js).

## Usage
```rust
extern crate sparse_bitfield as bitfield;
extern crate hypercore;

use hypercore::tree_index::TreeIndex;
use self::bitfield::{Bitfield, Change};

let bitfield = Bitfield::new(1024);
let mut tree = TreeIndex::new(bitfield);
assert_eq!(tree.set(0), Change::Changed);
assert_eq!(tree.set(0), Change::Unchanged);
assert_eq!(tree.get(0), true);
assert_eq!(tree.get(1), false);
```
