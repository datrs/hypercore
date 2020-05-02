//! Save data to a desired storage backend.

mod node;
mod persist;

pub use self::node::Node;
pub use self::persist::Persist;
pub use merkle_tree_stream::Node as NodeTrait;

use crate::Result;
use ed25519_dalek::{PublicKey, SecretKey, Signature, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use failure::Error;
use flat_tree as flat;
use random_access_disk::RandomAccessDisk;
use random_access_memory::RandomAccessMemory;
use random_access_storage::RandomAccess;
use sleep_parser::*;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::ops::Range;
use std::path::PathBuf;

const HEADER_OFFSET: u64 = 32;

#[derive(Debug)]
pub struct PartialKeypair {
    pub public: PublicKey,
    pub secret: Option<SecretKey>,
}

/// The types of stores that can be created.
#[derive(Debug)]
pub enum Store {
    /// Tree
    Tree,
    /// Data
    Data,
    /// Bitfield
    Bitfield,
    /// Signatures
    Signatures,
    /// Keypair
    Keypair,
}

/// Save data to a desired storage backend.
#[derive(Debug)]
pub struct Storage<T>
where
    T: RandomAccess + Debug,
{
    tree: T,
    data: T,
    bitfield: T,
    signatures: T,
    keypair: T,
}

impl<T> Storage<T>
where
    T: RandomAccess<Error = Error> + Debug,
{
    /// Create a new instance. Takes a keypair and a callback to create new
    /// storage instances.
    // Named `.open()` in the JS version. Replaces the `.openKey()` method too by
    // requiring a key pair to be initialized before creating a new instance.
    pub fn new<Cb>(create: Cb) -> Result<Self>
    where
        Cb: Fn(Store) -> Result<T>,
    {
        let mut instance = Self {
            tree: create(Store::Tree)?,
            data: create(Store::Data)?,
            bitfield: create(Store::Bitfield)?,
            signatures: create(Store::Signatures)?,
            keypair: create(Store::Keypair)?,
        };

        let header = create_bitfield();
        instance.bitfield.write(0, &header.to_vec())?;

        let header = create_signatures();
        instance.signatures.write(0, &header.to_vec())?;

        let header = create_tree();
        instance.tree.write(0, &header.to_vec())?;

        Ok(instance)
    }

    /// Write data to the feed.
    #[inline]
    pub fn write_data(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        self.data.write(offset, &data)
    }

    /// Write a byte vector to a data storage (random-access instance) at the
    /// position of `index`.
    ///
    /// NOTE: Meant to be called from the `.put()` feed method. Probably used to
    /// insert data as-is after receiving it from the network (need to confirm
    /// with mafintosh).
    /// TODO: Ensure the signature size is correct.
    /// NOTE: Should we create a `Data` entry type?
    pub fn put_data(&mut self, index: u64, data: &[u8], nodes: &[Node]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let range = self.data_offset(index, nodes)?;

        ensure!(
            (range.end - range.start) as usize == data.len(),
            format!("length  `{:?} != {:?}`", range.count(), data.len())
        );

        self.data.write(range.start, data)
    }

    /// Get data from disk that the user has written to it. This is stored
    /// unencrypted, so there's no decryption needed.
    // FIXME: data_offset always reads out index 0, length 0
    #[inline]
    pub fn get_data(&mut self, index: u64) -> Result<Vec<u8>> {
        let cached_nodes = Vec::new(); // TODO: reuse allocation.
        let range = self.data_offset(index, &cached_nodes)?;
        self.data.read(range.start, range.count() as u64)
    }

    /// Search the signature stores for a `Signature`, starting at `index`.
    pub fn next_signature(&mut self, index: u64) -> Result<Signature> {
        let bytes = self.signatures.read(HEADER_OFFSET + 64 * index, 64)?;
        if not_zeroes(&bytes) {
            Ok(Signature::from_bytes(&bytes)?)
        } else {
            Ok(self.next_signature(index + 1)?)
        }
    }

    /// Get a `Signature` from the store.
    #[inline]
    pub fn get_signature(&mut self, index: u64) -> Result<Signature> {
        let bytes = self.signatures.read(HEADER_OFFSET + 64 * index, 64)?;
        ensure!(not_zeroes(&bytes), "No signature found");
        Ok(Signature::from_bytes(&bytes)?)
    }

    /// Write a `Signature` to `self.Signatures`.
    /// TODO: Ensure the signature size is correct.
    /// NOTE: Should we create a `Signature` entry type?
    #[inline]
    pub fn put_signature(&mut self, index: u64, signature: impl Borrow<Signature>) -> Result<()> {
        let signature = signature.borrow();
        self.signatures
            .write(HEADER_OFFSET + 64 * index, &signature.to_bytes())
    }

    /// TODO(yw) docs
    /// Get the offset for the data, return `(offset, size)`.
    ///
    /// ## Panics
    /// A panic can occur if no maximum value is found.
    pub fn data_offset(&mut self, index: u64, cached_nodes: &[Node]) -> Result<Range<u64>> {
        let mut roots = Vec::new(); // TODO: reuse alloc
        flat::full_roots(tree_index(index), &mut roots);

        let mut offset = 0;
        let mut pending = roots.len() as u64;
        let block_index = tree_index(index);

        if pending == 0 {
            let len = match find_node(&cached_nodes, block_index) {
                Some(node) => node.len(),
                None => (self.get_node(block_index)?).len(),
            };
            return Ok(offset..offset + len);
        }

        for root in roots {
            // FIXME: we're always having a cache miss here. Check cache first before
            // getting a node from the backend.
            //
            // ```rust
            // let node = match find_node(cached_nodes, root) {
            //   Some(node) => node,
            //   None => self.get_node(root),
            // };
            // ```
            let node = self.get_node(root)?;

            offset += node.len();
            pending -= 1;
            if pending > 0 {
                continue;
            }

            let len = match find_node(&cached_nodes, block_index) {
                Some(node) => node.len(),
                None => (self.get_node(block_index)?).len(),
            };

            return Ok(offset..offset + len);
        }

        unreachable!();
    }

    /// Get a `Node` from the `tree` storage.
    #[inline]
    pub fn get_node(&mut self, index: u64) -> Result<Node> {
        let buf = self.tree.read(HEADER_OFFSET + 40 * index, 40)?;
        let node = Node::from_bytes(index, &buf)?;
        Ok(node)
    }

    /// Write a `Node` to the `tree` storage.
    /// TODO: prevent extra allocs here. Implement a method on node that can reuse
    /// a buffer.
    #[inline]
    pub fn put_node(&mut self, node: &Node) -> Result<()> {
        let index = node.index();
        let buf = node.to_bytes()?;
        self.tree.write(HEADER_OFFSET + 40 * index, &buf)
    }

    /// Write data to the internal bitfield module.
    /// TODO: Ensure the chunk size is correct.
    /// NOTE: Should we create a bitfield entry type?
    #[inline]
    pub fn put_bitfield(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        self.bitfield.write(HEADER_OFFSET + offset, data)
    }

    /// Read a public key from storage
    pub fn read_public_key(&mut self) -> Result<PublicKey> {
        let buf = self.keypair.read(0, PUBLIC_KEY_LENGTH as u64)?;
        let public_key = PublicKey::from_bytes(&buf)?;
        Ok(public_key)
    }

    /// Read a secret key from storage
    pub fn read_secret_key(&mut self) -> Result<SecretKey> {
        let buf = self
            .keypair
            .read(PUBLIC_KEY_LENGTH as u64, SECRET_KEY_LENGTH as u64)?;
        let secret_key = SecretKey::from_bytes(&buf)?;
        Ok(secret_key)
    }

    /// Write a public key to the storage
    pub fn write_public_key(&mut self, public_key: &PublicKey) -> Result<()> {
        let buf: [u8; PUBLIC_KEY_LENGTH] = public_key.to_bytes();
        self.keypair.write(0, &buf)
    }

    /// Write a secret key to the storage
    pub fn write_secret_key(&mut self, secret_key: &SecretKey) -> Result<()> {
        let buf: [u8; SECRET_KEY_LENGTH] = secret_key.to_bytes();
        self.keypair.write(PUBLIC_KEY_LENGTH as u64, &buf)
    }

    /// Tries to read a partial keypair (ie: with an optional secret_key) from the storage
    pub fn read_partial_keypair(&mut self) -> Option<PartialKeypair> {
        match self.read_public_key() {
            Ok(public) => match self.read_secret_key() {
                Ok(secret) => Some(PartialKeypair {
                    public,
                    secret: Some(secret),
                }),
                Err(_) => Some(PartialKeypair {
                    public,
                    secret: None,
                }),
            },
            Err(_) => None,
        }
    }
}

impl Storage<RandomAccessMemory> {
    /// Create a new instance backed by a `RandomAccessMemory` instance.
    pub fn new_memory() -> Result<Self> {
        let create = |_| Ok(RandomAccessMemory::default());
        Ok(Self::new(create)?)
    }
}

impl Storage<RandomAccessDisk> {
    /// Create a new instance backed by a `RandomAccessDisk` instance.
    pub fn new_disk(dir: &PathBuf) -> Result<Self> {
        let storage = |storage: Store| {
            let name = match storage {
                Store::Tree => "tree",
                Store::Data => "data",
                Store::Bitfield => "bitfield",
                Store::Signatures => "signatures",
                Store::Keypair => "key",
            };
            RandomAccessDisk::open(dir.as_path().join(name))
        };
        Ok(Self::new(storage)?)
    }
}

/// Get a node from a vector of nodes.
#[inline]
fn find_node(nodes: &[Node], index: u64) -> Option<&Node> {
    for node in nodes {
        if node.index() == index {
            return Some(node);
        }
    }
    None
}

/// Check if a byte slice is not completely zero-filled.
#[inline]
fn not_zeroes(bytes: &[u8]) -> bool {
    for byte in bytes {
        if *byte != 0 {
            return true;
        }
    }
    false
}

/// Convert the index to the index in the tree.
#[inline]
fn tree_index(index: u64) -> u64 {
    2 * index
}

#[test]
fn should_detect_zeroes() {
    let nums = vec![0; 10];
    assert!(!not_zeroes(&nums));

    let nums = vec![1; 10];
    assert!(not_zeroes(&nums));
}
