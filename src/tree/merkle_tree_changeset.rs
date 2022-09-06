use ed25519_dalek::{PublicKey, SecretKey, Signature};

use crate::{
    crypto::{signable_tree, Hash},
    sign, Node,
};

/// Changeset for a `MerkleTree`. This allows to incrementally change a `MerkleTree` in two steps:
/// first create the changes to this changeset, get out information from this to put to the oplog,
/// and the commit the changeset to the tree.
///
/// This is called "MerkleTreeBatch" in Javascript, see:
/// https://github.com/hypercore-protocol/hypercore/blob/master/lib/merkle-tree.js
#[derive(Debug)]
pub struct MerkleTreeChangeset {
    pub(crate) length: u64,
    pub(crate) byte_length: u64,
    pub(crate) fork: u64,
    pub(crate) roots: Vec<Node>,
    pub(crate) nodes: Vec<Node>,
    pub(crate) signature: Option<Signature>,
}

impl MerkleTreeChangeset {
    pub fn new(length: u64, byte_length: u64, fork: u64, roots: Vec<Node>) -> MerkleTreeChangeset {
        Self {
            length,
            byte_length,
            fork,
            roots,
            nodes: vec![],
            signature: None,
        }
    }

    pub fn append(&mut self, data: &[u8]) -> usize {
        let len = data.len();
        let head = self.length * 2;
        let iter = flat_tree::Iterator::new(head);
        let node = Node::new(head, Hash::data(data).as_bytes().to_vec(), len as u64);
        self.append_root(node, iter);
        len
    }

    pub fn append_root(&mut self, node: Node, iter: flat_tree::Iterator) {
        self.length += iter.factor() / 2;
        self.byte_length += node.length;
        self.roots.push(node.clone());
        self.nodes.push(node);

        // TODO: Javascript has this
        //
        // while (this.roots.length > 1) {
        //   const a = this.roots[this.roots.length - 1]
        //   const b = this.roots[this.roots.length - 2]

        //   // TODO: just have a peek sibling instead? (pretty sure it's always the left sib as well)
        //   if (ite.sibling() !== b.index) {
        //     ite.sibling() // unset so it always points to last root
        //     break
        //   }

        //   const node = parentNode(this.tree.crypto, ite.parent(), a, b)
        //   this.nodes.push(node)
        //   this.roots.pop()
        //   this.roots.pop()
        //   this.roots.push(node)
        // }
        // }
    }

    /// Hashes and signs the changeset
    pub fn hash_and_sign(&mut self, public_key: &PublicKey, secret_key: &SecretKey) -> Box<[u8]> {
        let hash = self.hash();
        let signable = signable_tree(&hash, self.length, self.fork);
        let signature = sign(&public_key, &secret_key, &signable);
        self.signature = Some(signature);
        hash
    }

    /// Calculates a hash of the current set of roots
    pub fn hash(&self) -> Box<[u8]> {
        Hash::tree(&self.roots).as_bytes().into()
    }
}
