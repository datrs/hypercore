use blake2::{
    digest::{generic_array::GenericArray, typenum::U32, FixedOutput},
    Blake2b, Blake2bMac, Digest,
};
use byteorder::{BigEndian, WriteBytesExt};
use compact_encoding::State;
use ed25519_dalek::VerifyingKey;
use merkle_tree_stream::Node as NodeTrait;
use std::convert::AsRef;
use std::mem;
use std::ops::{Deref, DerefMut};

use crate::common::Node;

// https://en.wikipedia.org/wiki/Merkle_tree#Second_preimage_attack
const LEAF_TYPE: [u8; 1] = [0x00];
const PARENT_TYPE: [u8; 1] = [0x01];
const ROOT_TYPE: [u8; 1] = [0x02];
const HYPERCORE: [u8; 9] = *b"hypercore";

// These the output of, see `hash_namespace` test below for how they are produced
// https://github.com/holepunchto/hypercore/blob/cf08b72f14ed7d9ef6d497ebb3071ee0ae20967e/lib/caps.js#L16
const TREE: [u8; 32] = [
    0x9F, 0xAC, 0x70, 0xB5, 0xC, 0xA1, 0x4E, 0xFC, 0x4E, 0x91, 0xC8, 0x33, 0xB2, 0x4, 0xE7, 0x5B,
    0x8B, 0x5A, 0xAD, 0x8B, 0x58, 0x81, 0xBF, 0xC0, 0xAD, 0xB5, 0xEF, 0x38, 0xA3, 0x27, 0x5B, 0x9C,
];

// const DEFAULT_NAMESPACE: [u8; 32] = [
//     0x41, 0x44, 0xEE, 0xA5, 0x31, 0xE4, 0x83, 0xD5, 0x4E, 0x0C, 0x14, 0xF4, 0xCA, 0x68, 0xE0, 0x64,
//     0x4F, 0x35, 0x53, 0x43, 0xFF, 0x6F, 0xCB, 0x0F, 0x00, 0x52, 0x00, 0xE1, 0x2C, 0xD7, 0x47, 0xCB,
// ];

// const MANIFEST: [u8; 32] = [
//     0xE6, 0x4B, 0x71, 0x08, 0xEA, 0xCC, 0xE4, 0x7C, 0xFC, 0x61, 0xAC, 0x85, 0x05, 0x68, 0xF5, 0x5F,
//     0x8B, 0x15, 0xB8, 0x2E, 0xC5, 0xED, 0x78, 0xC4, 0xEC, 0x59, 0x7B, 0x03, 0x6E, 0x2A, 0x14, 0x98,
// ];

pub(crate) type Blake2bResult = GenericArray<u8, U32>;
type Blake2b256 = Blake2b<U32>;

/// `BLAKE2b` hash.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Hash {
    hash: Blake2bResult,
}

impl Hash {
    /// Hash a `Leaf` node.
    #[allow(dead_code)]
    pub(crate) fn from_leaf(data: &[u8]) -> Self {
        let size = u64_as_be(data.len() as u64);

        let mut hasher = Blake2b256::new();
        hasher.update(LEAF_TYPE);
        hasher.update(size);
        hasher.update(data);

        Self {
            hash: hasher.finalize(),
        }
    }

    /// Hash two `Leaf` nodes hashes together to form a `Parent` hash.
    #[allow(dead_code)]
    pub(crate) fn from_hashes(left: &Node, right: &Node) -> Self {
        let (node1, node2) = if left.index <= right.index {
            (left, right)
        } else {
            (right, left)
        };

        let size = u64_as_be(node1.length + node2.length);

        let mut hasher = Blake2b256::new();
        hasher.update(PARENT_TYPE);
        hasher.update(size);
        hasher.update(node1.hash());
        hasher.update(node2.hash());

        Self {
            hash: hasher.finalize(),
        }
    }

    /// Hash a public key. Useful to find the key you're looking for on a public
    /// network without leaking the key itself.
    #[allow(dead_code)]
    pub(crate) fn for_discovery_key(public_key: VerifyingKey) -> Self {
        let mut hasher =
            Blake2bMac::<U32>::new_with_salt_and_personal(public_key.as_bytes(), &[], &[]).unwrap();
        blake2::digest::Update::update(&mut hasher, &HYPERCORE);
        Self {
            hash: hasher.finalize_fixed(),
        }
    }

    /// Hash a vector of `Root` nodes.
    // Called `crypto.tree()` in the JS implementation.
    #[allow(dead_code)]
    pub(crate) fn from_roots(roots: &[impl AsRef<Node>]) -> Self {
        let mut hasher = Blake2b256::new();
        hasher.update(ROOT_TYPE);

        for node in roots {
            let node = node.as_ref();
            hasher.update(node.hash());
            hasher.update(u64_as_be(node.index()));
            hasher.update(u64_as_be(node.len()));
        }

        Self {
            hash: hasher.finalize(),
        }
    }

    /// Returns a byte slice of this `Hash`'s contents.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.hash.as_slice()
    }

    // NB: The following methods mirror Javascript naming in
    // https://github.com/mafintosh/hypercore-crypto/blob/master/index.js
    // for v10 that use LE bytes.

    /// Hash data
    pub(crate) fn data(data: &[u8]) -> Self {
        let (mut state, mut size) = State::new_with_size(8);
        state
            .encode_u64(data.len() as u64, &mut size)
            .expect("Encoding u64 should not fail");

        let mut hasher = Blake2b256::new();
        hasher.update(LEAF_TYPE);
        hasher.update(&size);
        hasher.update(data);

        Self {
            hash: hasher.finalize(),
        }
    }

    /// Hash a parent
    pub(crate) fn parent(left: &Node, right: &Node) -> Self {
        let (node1, node2) = if left.index <= right.index {
            (left, right)
        } else {
            (right, left)
        };

        let (mut state, mut size) = State::new_with_size(8);
        state
            .encode_u64(node1.length + node2.length, &mut size)
            .expect("Encoding u64 should not fail");

        let mut hasher = Blake2b256::new();
        hasher.update(PARENT_TYPE);
        hasher.update(&size);
        hasher.update(node1.hash());
        hasher.update(node2.hash());

        Self {
            hash: hasher.finalize(),
        }
    }

    /// Hash a tree
    pub(crate) fn tree(roots: &[impl AsRef<Node>]) -> Self {
        let mut hasher = Blake2b256::new();
        hasher.update(ROOT_TYPE);

        for node in roots {
            let node = node.as_ref();
            let (mut state, mut buffer) = State::new_with_size(16);
            state
                .encode_u64(node.index(), &mut buffer)
                .expect("Encoding u64 should not fail");
            state
                .encode_u64(node.len(), &mut buffer)
                .expect("Encoding u64 should not fail");

            hasher.update(node.hash());
            hasher.update(&buffer[..8]);
            hasher.update(&buffer[8..]);
        }

        Self {
            hash: hasher.finalize(),
        }
    }
}

fn u64_as_be(n: u64) -> [u8; 8] {
    let mut size = [0u8; mem::size_of::<u64>()];
    size.as_mut().write_u64::<BigEndian>(n).unwrap();
    size
}

impl Deref for Hash {
    type Target = Blake2bResult;

    fn deref(&self) -> &Self::Target {
        &self.hash
    }
}

impl DerefMut for Hash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.hash
    }
}

/// Create a signable buffer for tree. This is treeSignable in Javascript.
/// See https://github.com/hypercore-protocol/hypercore/blob/70b271643c4e4b1e5ecae5bb579966dfe6361ff3/lib/caps.js#L17
pub(crate) fn signable_tree(hash: &[u8], length: u64, fork: u64) -> Box<[u8]> {
    let (mut state, mut buffer) = State::new_with_size(80);
    state
        .encode_fixed_32(&TREE, &mut buffer)
        .expect("Should be able ");
    state
        .encode_fixed_32(hash, &mut buffer)
        .expect("Encoding fixed 32 bytes should not fail");
    state
        .encode_u64(length, &mut buffer)
        .expect("Encoding u64 should not fail");
    state
        .encode_u64(fork, &mut buffer)
        .expect("Encoding u64 should not fail");
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    use self::data_encoding::HEXLOWER;
    use data_encoding;

    fn hash_with_extra_byte(data: &[u8], byte: u8) -> Box<[u8]> {
        let mut hasher = Blake2b256::new();
        hasher.update(data);
        hasher.update([byte]);
        let hash = hasher.finalize();
        hash.as_slice().into()
    }

    fn hex_bytes(hex: &str) -> Vec<u8> {
        HEXLOWER.decode(hex.as_bytes()).unwrap()
    }

    fn check_hash(hash: Hash, hex: &str) {
        assert_eq!(hash.as_bytes(), &hex_bytes(hex)[..]);
    }

    #[test]
    fn leaf_hash() {
        check_hash(
            Hash::from_leaf(&[]),
            "5187b7a8021bf4f2c004ea3a54cfece1754f11c7624d2363c7f4cf4fddd1441e",
        );
        check_hash(
            Hash::from_leaf(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
            "e1001bb0bb9322b6b202b2f737dc12181b11727168d33ca48ffe361c66cd1abe",
        );
    }

    #[test]
    fn parent_hash() {
        let d1: &[u8] = &[0, 1, 2, 3, 4];
        let d2: &[u8] = &[42, 43, 44, 45, 46, 47, 48];
        let node1 = Node::new(0, Hash::from_leaf(d1).as_bytes().to_vec(), d1.len() as u64);
        let node2 = Node::new(1, Hash::from_leaf(d2).as_bytes().to_vec(), d2.len() as u64);
        check_hash(
            Hash::from_hashes(&node1, &node2),
            "6fac58578fa385f25a54c0637adaca71fdfddcea885d561f33d80c4487149a14",
        );
        check_hash(
            Hash::from_hashes(&node2, &node1),
            "6fac58578fa385f25a54c0637adaca71fdfddcea885d561f33d80c4487149a14",
        );
    }

    #[test]
    fn root_hash() {
        let d1: &[u8] = &[0, 1, 2, 3, 4];
        let d2: &[u8] = &[42, 43, 44, 45, 46, 47, 48];
        let node1 = Node::new(0, Hash::from_leaf(d1).as_bytes().to_vec(), d1.len() as u64);
        let node2 = Node::new(1, Hash::from_leaf(d2).as_bytes().to_vec(), d2.len() as u64);
        check_hash(
            Hash::from_roots(&[&node1, &node2]),
            "2d117e0bb15c6e5236b6ce764649baed1c41890da901a015341503146cc20bcd",
        );
        check_hash(
            Hash::from_roots(&[&node2, &node1]),
            "9826c8c2d28fc309cce73a4b6208e83e5e4b0433d2369bfbf8858272153849f1",
        );
    }

    #[test]
    fn discovery_key_hashing() -> Result<(), ed25519_dalek::SignatureError> {
        let public_key = VerifyingKey::from_bytes(&[
            119, 143, 141, 149, 81, 117, 201, 46, 76, 237, 94, 79, 85, 99, 246, 155, 254, 192, 200,
            108, 198, 246, 112, 53, 44, 69, 121, 67, 102, 111, 230, 57,
        ])?;

        let expected = &[
            37, 167, 138, 168, 22, 21, 132, 126, 186, 0, 153, 93, 242, 157, 212, 29, 126, 227, 15,
            59, 1, 248, 146, 32, 159, 121, 183, 90, 87, 217, 137, 225,
        ];

        assert_eq!(Hash::for_discovery_key(public_key).as_bytes(), expected);

        Ok(())
    }

    // The following uses test data from
    // https://github.com/mafintosh/hypercore-crypto/blob/master/test.js

    #[test]
    fn hash_leaf() {
        let data = b"hello world";
        check_hash(
            Hash::data(data),
            "9f1b578fd57a4df015493d2886aec9600eef913c3bb009768c7f0fb875996308",
        );
    }

    #[test]
    fn hash_parent() {
        let data = b"hello world";
        let len = data.len() as u64;
        let node1 = Node::new(0, Hash::data(data).as_bytes().to_vec(), len);
        let node2 = Node::new(1, Hash::data(data).as_bytes().to_vec(), len);
        check_hash(
            Hash::parent(&node1, &node2),
            "3ad0c9b58b771d1b7707e1430f37c23a23dd46e0c7c3ab9c16f79d25f7c36804",
        );
    }

    #[test]
    fn hash_tree() {
        let hash: [u8; 32] = [0; 32];
        let node1 = Node::new(3, hash.to_vec(), 11);
        let node2 = Node::new(9, hash.to_vec(), 2);
        check_hash(
            Hash::tree(&[&node1, &node2]),
            "0e576a56b478cddb6ffebab8c494532b6de009466b2e9f7af9143fc54b9eaa36",
        );
    }

    // This is the rust version from
    // https://github.com/hypercore-protocol/hypercore/blob/70b271643c4e4b1e5ecae5bb579966dfe6361ff3/lib/caps.js
    // and validates that our arrays match
    #[test]
    fn hash_namespace() {
        let mut hasher = Blake2b256::new();
        hasher.update(HYPERCORE);
        let hash = hasher.finalize();
        let ns = hash.as_slice();
        let tree: Box<[u8]> = { hash_with_extra_byte(ns, 0) };
        assert_eq!(tree, TREE.into());
    }
}
