//! Hypercore's main abstraction. Exposes an append-only, secure log structure.

pub use crypto::Keypair;
pub use feed_builder::FeedBuilder;
pub use storage::{Node, NodeTrait, Storage, Store};

use crypto::{generate_keypair, sign, verify, Hash, Merkle, Signature};
use flat_tree as flat;
use proof::Proof;
use random_access_disk::{RandomAccessDisk, RandomAccessDiskMethods};
use random_access_memory::{RandomAccessMemory, RandomAccessMemoryMethods};
use random_access_storage::RandomAccessMethods;
use sparse_bitfield::Bitfield;
use tree_index::TreeIndex;
use Result;

use std::cmp;
use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;

/// Append-only log structure.
pub struct Feed<T>
where
  T: RandomAccessMethods + Debug,
{
  /// Merkle tree instance.
  pub(crate) merkle: Merkle,
  /// Ed25519 key pair.
  pub(crate) keypair: Keypair,
  /// Struct that saves data to a `random-access-storage` backend.
  pub(crate) storage: Storage<T>,
  /// Total length of data stored.
  pub(crate) byte_length: usize,
  /// TODO: description. Length of... roots?
  pub(crate) length: usize,
  /// Bitfield to keep track of which data we own.
  pub(crate) bitfield: Bitfield,
  pub(crate) tree: TreeIndex,
}

impl<T> Feed<T>
where
  T: RandomAccessMethods + Debug,
{
  /// Create a new instance with a custom storage backend.
  pub fn with_storage(storage: ::storage::Storage<T>) -> Result<Self> {
    let keypair = generate_keypair(); // TODO: read key pair from disk;
    Ok(FeedBuilder::new(keypair, storage).build()?)
  }

  /// Get the amount of entries in the feed.
  #[inline]
  pub fn len(&self) -> usize {
    self.length
  }

  /// Check if the length is 0.
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Get the total amount of bytes stored in the feed.
  pub fn byte_len(&self) -> usize {
    self.byte_length
  }

  /// Append data into the log.
  pub fn append(&mut self, data: &[u8]) -> Result<()> {
    self.merkle.next(data);
    let mut offset = 0;

    self.storage.write_data(self.byte_length + offset, &data)?;
    offset += data.len();

    let hash = Hash::from_roots(self.merkle.roots());
    let index = self.length;
    let signature = sign(&self.keypair, hash.as_bytes());
    self.storage.put_signature(index, signature)?;

    for node in self.merkle.nodes() {
      self.storage.put_node(node)?;
    }

    self.byte_length += offset;

    self.bitfield.set(self.length, true);
    self.tree.set(2 * self.length);
    self.length += 1;

    Ok(())
  }

  /// Retrieve data from the log.
  pub fn get(&mut self, index: usize) -> Result<Option<Vec<u8>>> {
    if !self.bitfield.get(index) {
      // NOTE: Do (network) lookup here once we have network code.
      return Ok(None);
    }
    Ok(Some(self.storage.get_data(index)?))
  }

  /// Return the Nodes which prove the correctness for the Node at index.
  pub fn proof(&mut self, index: usize) -> Result<Proof> {
    let mut remote_tree = TreeIndex::default();
    let mut nodes = vec![];
    let proof = self.tree.proof(2 * index, &mut nodes, &mut remote_tree);

    if let Some(proof) = proof {
      let signature = self.storage.get_signature(proof.verified_by() / 2 - 1)?;
      let mut nodes = Vec::with_capacity(proof.nodes().len());
      for index in proof.nodes() {
        let node = self.storage.get_node(*index)?;
        nodes.push(node);
      }

      Ok(Proof {
        nodes,
        signature,
        index,
      })
    } else {
      bail!("No proof available for index {}", index);
    }
  }

  /// Insert data into the tree at `index`. Verifies the `proof` when inserting
  /// to make sure data is correct. Useful when replicating data from a remote
  /// host.
  pub fn put(
    &mut self,
    index: usize,
    data: Option<&[u8]>,
    mut proof: Proof,
  ) -> Result<()> {
    let mut next = 2 * index;
    let mut trusted: Option<usize> = None;
    let mut missing = vec![];
    let mut i = 0;

    loop {
      if self.tree.get(next) {
        trusted = Some(next);
        break;
      }

      let sibling = flat::sibling(next);
      next = flat::parent(next);

      if i < proof.nodes.len() && proof.nodes[i].index == sibling {
        i += 1;
        continue;
      }

      if !self.tree.get(sibling) {
        break;
      }
      missing.push(sibling);
    }

    if trusted.is_none() && self.tree.get(next) {
      trusted = Some(next);
    }

    let mut missing_nodes = vec![];
    for index in missing {
      let node = self.storage.get_node(index)?;
      missing_nodes.push(node);
    }

    let mut trusted_node = None;
    if let Some(index) = trusted {
      let node = self.storage.get_node(index)?;
      trusted_node = Some(node);
    }

    let mut visited = vec![];
    let mut top = match data {
      None => proof.nodes.remove(0),
      Some(data) => Node::new(
        2 * index,
        Hash::from_leaf(&data).as_bytes().to_owned(),
        data.len(),
      ),
    };

    let verify_node = |trusted: &Option<Node>, node: &Node| -> bool {
      match trusted {
        None => false,
        Some(trusted) => {
          trusted.index == node.index && trusted.hash == node.hash
        }
      }
    };

    // check if we already have the hash for this node
    if verify_node(&trusted_node, &top) {
      self.write(index, data, &visited, None)?;
      return Ok(());
    }

    // keep hashing with siblings until we reach the end or trusted node
    loop {
      let node;
      let next = flat::sibling(top.index);

      if !proof.nodes.is_empty() && proof.nodes[0].index == next {
        node = proof.nodes.remove(0);
        visited.push(node.clone());
      } else if !missing_nodes.is_empty() && missing_nodes[0].index == next {
        node = missing_nodes.remove(0);
      } else {
        let nodes = self.verify_roots(&top, &mut proof)?;
        visited.extend_from_slice(&nodes);
        self.write(index, data, &visited, Some(&proof.signature))?;
        return Ok(());
      }

      visited.push(top.clone());
      let hash = Hash::from_hashes(&top.hash, &node.hash);
      let len = top.len() + node.len();
      top = Node::new(flat::parent(top.index), hash.as_bytes().into(), len);

      if verify_node(&trusted_node, &top) {
        self.write(index, data, &visited, None)?;
        return Ok(());
      }
    }
  }

  /// Write some data to disk. Usually used in combination with `.put()`.
  // in JS this calls to:
  // - ._write()
  // - ._onwrite() (emit the 'write' event), if it exists
  // - ._writeAfterHook() (optionally going through writeHookdone())
  // - ._writeDone()
  //
  // Arguments are: (index, data, node, sig, from, cb)
  fn write(
    &mut self,
    index: usize,
    data: Option<&[u8]>,
    nodes: &[Node],
    sig: Option<&Signature>,
  ) -> Result<()> {
    for node in nodes {
      self.storage.put_node(node)?;
    }

    if let Some(data) = data {
      self.storage.put_data(index, data, &nodes)?;
    }

    if let Some(sig) = sig {
      self.storage.put_signature(index, sig)?;
    }

    for node in nodes {
      self.tree.set(node.index);
    }

    self.tree.set(2 * index);

    if let Some(_data) = data {
      if self.bitfield.set(index, true).is_changed() {
        // TODO: emit "download" event
      }
      // TODO: check peers.length, call ._announce if peers exist.
    }

    // TODO: Discern between "primary" and "replica" streams.
    // if (!this.writable) {
    //   if (!this._synced) this._synced = this.bitfield.iterator(0, this.length)
    //   if (this._synced.next() === -1) {
    //     this._synced.range(0, this.length)
    //     this._synced.seek(0)
    //     if (this._synced.next() === -1) {
    //       this.emit('sync')
    //     }
    //   }
    // }

    Ok(())
  }

  /// Get a signature from the store.
  pub fn signature(&mut self, index: usize) -> Result<Signature> {
    ensure!(
      index < self.length,
      format!("No signature found for index {}", index)
    );
    Ok(self.storage.next_signature(index)?)
  }

  /// Verify the entire feed. Checks a signature against the signature of all
  /// root nodes combined.
  pub fn verify(&mut self, index: usize, signature: &Signature) -> Result<()> {
    let roots = self.root_hashes(index)?;
    let roots: Vec<_> = roots.into_iter().map(Rc::new).collect();

    let message = Hash::from_roots(&roots);
    let message = message.as_bytes();

    verify(&self.keypair.public, message, signature)?;
    Ok(())
  }

  /// Get all root hashes from the feed.
  // In the JavaScript implementation this calls to `._getRootsToVerify()`
  // internally. In Rust it seems better to just inline the code.
  pub fn root_hashes(&mut self, index: usize) -> Result<Vec<Node>> {
    ensure!(
      index <= self.length,
      format!("Root index bounds exceeded {} > {}", index, self.length)
    );
    let roots_index = index * 2 + 2;
    let mut indexes = vec![];
    flat::full_roots(roots_index, &mut indexes);

    let mut roots = Vec::with_capacity(indexes.len());
    for index in indexes {
      let node = self.storage.get_node(index)?;
      roots.push(node);
    }

    Ok(roots)
  }

  /// Access the keypair.
  pub fn keypair(&self) -> &Keypair {
    &self.keypair
  }

  fn verify_roots(
    &mut self,
    top: &Node,
    proof: &mut Proof,
  ) -> Result<Vec<Node>> {
    let last_node = if !proof.nodes.is_empty() {
      proof.nodes[proof.nodes.len() - 1].index
    } else {
      top.index
    };

    let verified_by =
      cmp::max(flat::right_span(top.index), flat::right_span(last_node)) + 2;

    // `Feed.prototype._getRootsToVerify in the JS implementation`
    let mut indexes = vec![];
    flat::full_roots(verified_by, &mut indexes);
    let mut roots = Vec::with_capacity(indexes.len());
    let mut extra_nodes = vec![];

    for index in indexes {
      if index == top.index {
        extra_nodes.push(top.clone());
        roots.push(top.clone()); // TODO: verify this is the right index to push to.
      } else if !proof.nodes.is_empty() && index == proof.nodes[0].index {
        extra_nodes.push(proof.nodes[0].clone());
        roots.push(proof.nodes.remove(0)); // TODO: verify this is the right index to push to.
      } else if self.tree.get(index) {
        let node = self.storage.get_node(index)?;
        roots.push(node);
      } else {
        bail!("<hypercore>: Missing tree roots needed for verify");
      }
    }

    let checksum = Hash::from_roots(&roots);
    verify(&self.keypair.public, checksum.as_bytes(), &proof.signature)?;

    // Update the length if we grew the feed.
    let len = verified_by / 2;
    if len > self.len() {
      self.length = len;
      self.byte_length = roots.iter().fold(0, |acc, root| acc + root.index)
      // TODO: emit('append')
    }

    Ok(extra_nodes)
  }
}

impl Feed<RandomAccessDiskMethods> {
  /// Create a new instance that persists to disk at the location of `dir`.
  // TODO: Ensure that dir is always a directory.
  // NOTE: Should we `mkdirp` here?
  // NOTE: Should we call these `data.bitfield` / `data.tree`?
  pub fn new(dir: &PathBuf) -> Result<Self> {
    let create = |storage: Store| {
      let name = match storage {
        Store::Tree => "tree",
        Store::Data => "data",
        Store::Bitfield => "bitfield",
        Store::Signatures => "signatures",
      };
      RandomAccessDisk::new(dir.as_path().join(name))
    };

    let storage = Storage::new(create)?;
    let keypair = generate_keypair(); // TODO: read keypair from disk;
    Ok(FeedBuilder::new(keypair, storage).build()?)
  }
}

/// Create a new instance with an in-memory storage backend.
///
/// ## Panics
/// Can panic if constructing the in-memory store fails, which is highly
/// unlikely.
impl Default for Feed<RandomAccessMemoryMethods> {
  fn default() -> Self {
    let create = |_| RandomAccessMemory::default();
    let storage = Storage::new(create).unwrap();
    Self::with_storage(storage).unwrap()
  }
}
