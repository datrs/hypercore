//! Hypercore's main abstraction. Exposes an append-only, secure log structure.

use feed_builder::FeedBuilder;
use replicate::{Message, Peer};
pub use storage::{Node, NodeTrait, Storage, Store};

use bitfield::Bitfield;
use crypto::{generate_keypair, sign, verify, Hash, Merkle};
use ed25519_dalek::{PublicKey, SecretKey, Signature};
use failure::Error;
use flat_tree as flat;
use pretty_hash::fmt as pretty_fmt;
use proof::Proof;
use random_access_disk::RandomAccessDiskMethods;
use random_access_memory::RandomAccessMemoryMethods;
use random_access_storage::RandomAccessMethods;
use tree_index::TreeIndex;
use Result;

use std::borrow::Borrow;
use std::cmp;
use std::fmt::{self, Debug, Display};
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;

/// Append-only log structure.
#[derive(Debug)]
pub struct Feed<T>
where
  T: RandomAccessMethods<Error = Error> + Debug,
{
  /// Merkle tree instance.
  pub(crate) merkle: Merkle,
  pub(crate) public_key: PublicKey,
  pub(crate) secret_key: Option<SecretKey>,
  pub(crate) storage: Storage<T>,
  /// Total length of data stored.
  pub(crate) byte_length: usize,
  /// TODO: description. Length of... roots?
  pub(crate) length: usize,
  /// Bitfield to keep track of which data we own.
  pub(crate) bitfield: Bitfield,
  pub(crate) tree: TreeIndex,
  pub(crate) peers: Vec<Peer>,
}

impl<T> Feed<T>
where
  T: RandomAccessMethods<Error = Error> + Debug,
{
  /// Create a new instance with a custom storage backend.
  pub fn with_storage(mut storage: ::storage::Storage<T>) -> Result<Self> {
    match storage.read_partial_keypair() {
      Some(partial_keypair) => {
        let builder = FeedBuilder::new(partial_keypair.public, storage);

        // return early without secret key
        if partial_keypair.secret.is_none() {
          return Ok(builder.build()?);
        }

        Ok(
          builder
            .secret_key(partial_keypair.secret.unwrap())
            .build()?,
        )
      }
      None => {
        // we have no keys, generate a pair and save them to the storage
        let keypair = generate_keypair();
        storage.write_public_key(&keypair.public)?;
        storage.write_secret_key(&keypair.secret)?;

        Ok(
          FeedBuilder::new(keypair.public, storage)
            .secret_key(keypair.secret)
            .build()?,
        )
      }
    }
  }

  /// Starts a `FeedBuilder` with the provided `Keypair` and `Storage`.
  pub fn builder(public_key: PublicKey, storage: Storage<T>) -> FeedBuilder<T> {
    FeedBuilder::new(public_key, storage)
  }

  /// Get the amount of entries in the feed.
  #[inline]
  pub fn len(&self) -> usize {
    self.length
  }

  /// Check if the length is 0.
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Get the total amount of bytes stored in the feed.
  #[inline]
  pub fn byte_len(&self) -> usize {
    self.byte_length
  }

  /// Append data into the log.
  #[inline]
  pub fn append(&mut self, data: &[u8]) -> Result<()> {
    let key = match &self.secret_key {
      Some(key) => key,
      None => bail!("no secret key, cannot append."),
    };
    self.merkle.next(data);
    let mut offset = 0;

    self.storage.write_data(self.byte_length + offset, &data)?;
    offset += data.len();

    let hash = Hash::from_roots(self.merkle.roots());
    let index = self.length;
    let signature = sign(&self.public_key, key, hash.as_bytes());
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

  /// Get the block of data at the tip of the feed. This will be the most
  /// recently appended block.
  #[inline]
  pub fn head(&mut self) -> Result<Option<Vec<u8>>> {
    match self.len() {
      0 => Ok(None),
      len => self.get(len - 1),
    }
  }

  /// Return `true` if a data block is available locally.
  #[inline]
  pub fn has(&mut self, index: usize) -> bool {
    self.bitfield.get(index)
  }

  /// Return `true` if all data blocks within a range are available locally.
  #[inline]
  pub fn has_all(&mut self, range: ::std::ops::Range<usize>) -> bool {
    let total = range.len();
    total == self.bitfield.total_with_range(range) as usize
  }

  /// Get the total amount of chunks downloaded.
  #[inline]
  pub fn downloaded(&mut self, range: ::std::ops::Range<usize>) -> u8 {
    self.bitfield.total_with_range(range)
  }

  /// Retrieve data from the log.
  #[inline]
  pub fn get(&mut self, index: usize) -> Result<Option<Vec<u8>>> {
    if !self.bitfield.get(index) {
      // NOTE: Do (network) lookup here once we have network code.
      return Ok(None);
    }
    Ok(Some(self.storage.get_data(index)?))
  }

  /// Return the Nodes which prove the correctness for the Node at index.
  #[inline]
  pub fn proof(&mut self, index: usize, include_hash: bool) -> Result<Proof> {
    self.proof_with_digest(index, 0, include_hash)
  }

  pub fn proof_with_digest(
    &mut self,
    index: usize,
    digest: usize,
    include_hash: bool,
  ) -> Result<Proof> {
    let mut remote_tree = TreeIndex::default();
    let mut nodes = vec![];

    let proof = self.tree.proof_with_digest(
      2 * index,
      digest,
      include_hash,
      &mut nodes,
      &mut remote_tree,
    );

    let proof = match proof {
      Some(proof) => proof,
      None => bail!("No proof available for index {}", index),
    };

    let tmp_num = proof.verified_by() / 2;
    let (sig_index, has_underflow) = tmp_num.overflowing_sub(1);
    let signature = if has_underflow {
      None
    } else {
      match self.storage.get_signature(sig_index) {
        Ok(sig) => Some(sig),
        Err(_) => None,
      }
    };

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
  }

  /// Compute the digest for the index.
  pub fn digest(&mut self, index: usize) -> usize {
    self.tree.digest(2 * index)
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

    let mut i = match data {
      Some(_) => 0,
      None => 1,
    };

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
      Some(data) => Node::new(
        2 * index,
        Hash::from_leaf(&data).as_bytes().to_owned(),
        data.len(),
      ),
      None => proof.nodes.remove(0),
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
        // TODO: panics here
        let nodes = self.verify_roots(&top, &mut proof)?;
        visited.extend_from_slice(&nodes);
        self.write(index, data, &visited, proof.signature)?;
        return Ok(());
      }

      visited.push(top.clone());
      let hash = Hash::from_hashes(&top, &node);
      let len = top.len() + node.len();
      top = Node::new(flat::parent(top.index), hash.as_bytes().into(), len);

      if verify_node(&trusted_node, &top) {
        self.write(index, data, &visited, None)?;
        return Ok(());
      }
    }

    fn verify_node(trusted: &Option<Node>, node: &Node) -> bool {
      match trusted {
        None => false,
        Some(trusted) => {
          trusted.index == node.index && trusted.hash == node.hash
        }
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
    sig: Option<Signature>,
  ) -> Result<()> {
    for node in nodes {
      self.storage.put_node(node)?;
    }

    if let Some(data) = data {
      self.storage.put_data(index, data, &nodes)?;
    }

    if let Some(sig) = sig {
      let sig = sig.borrow();
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

    verify(&self.public_key, message, Some(signature))?;
    Ok(())
  }

  /// Announce we have a piece of data to all other peers.
  // TODO: probably shouldn't be public
  pub fn announce(&mut self, message: &Message, from: &Peer) {
    for peer in &mut self.peers {
      if peer != from {
        peer.have(message)
      }
    }
  }

  /// Announce we no longer have a piece of data to all other peers.
  // TODO: probably shouldn't be public
  pub fn unannounce(&mut self, message: &Message) {
    for peer in &mut self.peers {
      peer.unhave(message)
    }
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

  /// Access the public key.
  pub fn public_key(&self) -> &PublicKey {
    &self.public_key
  }

  /// Access the secret key.
  pub fn secret_key(&self) -> &Option<SecretKey> {
    &self.secret_key
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
    verify(&self.public_key, checksum.as_bytes(), proof.signature())?;

    // Update the length if we grew the feed.
    let len = verified_by / 2;
    if len > self.len() {
      self.length = len;
      self.byte_length = roots.iter().fold(0, |acc, root| acc + root.index)
      // TODO: emit('append')
    }

    Ok(extra_nodes)
  }

  /// (unimplemented) Provide a range of data to download.
  pub fn download(&mut self, _range: Range<usize>) -> Result<()> {
    unimplemented!();
  }

  /// (unimplemented) Provide a range of data to remove from the local storage.
  pub fn undownload(&mut self, _range: Range<usize>) -> Result<()> {
    unimplemented!();
  }

  /// (unimplemented) End the feed.
  pub fn finalize(&mut self) -> Result<()> {
    // if (!this.key) {
    //   this.key = crypto.tree(this._merkle.roots)
    //   this.discoveryKey = crypto.discoveryKey(this.key)
    // }
    // this._storage.key.write(0, this.key, cb)
    unimplemented!();
  }

  /// Update all peers.
  pub fn update_peers(&mut self) {
    for peer in &mut self.peers {
      peer.update();
    }
  }
}

impl Feed<RandomAccessDiskMethods> {
  /// Create a new instance that persists to disk at the location of `dir`.
  // TODO: Ensure that dir is always a directory.
  // NOTE: Should we `mkdirp` here?
  // NOTE: Should we call these `data.bitfield` / `data.tree`?
  pub fn new(dir: &PathBuf) -> Result<Self> {
    let storage = Storage::new_disk(&dir)?;
    Ok(Self::with_storage(storage)?)
  }
}

/// Create a new instance with an in-memory storage backend.
///
/// ## Panics
/// Can panic if constructing the in-memory store fails, which is highly
/// unlikely.
impl Default for Feed<RandomAccessMemoryMethods> {
  fn default() -> Self {
    let storage = Storage::new_memory().unwrap();
    Self::with_storage(storage).unwrap()
  }
}

impl<T: RandomAccessMethods<Error = Error> + Debug> Display for Feed<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    // TODO: yay, we should find a way to convert this .unwrap() to an error
    // type that's accepted by `fmt::Result<(), fmt::Error>`.
    let key = pretty_fmt(&self.public_key.to_bytes()).unwrap();
    let byte_len = self.byte_len();
    let len = self.len();
    let peers = 0; // TODO: update once we actually have peers.
    write!(
      f,
      "Hypercore(key=[{}], length={}, byte_length={}, peers={})",
      key, len, byte_len, peers
    )
  }
}
