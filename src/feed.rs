//! Hypercore's main abstraction. Exposes an append-only, secure log structure.

use crate::feed_builder::FeedBuilder;
use crate::replicate::{Message, Peer};
pub use crate::storage::{Node, NodeTrait, Storage, Store};

use crate::audit::Audit;
use crate::bitfield::Bitfield;
use crate::crypto::{generate_keypair, sign, verify, Hash, Merkle};
use crate::proof::Proof;
use anyhow::{bail, ensure, Result};
use ed25519_dalek::{PublicKey, SecretKey, Signature};
use flat_tree as flat;
use pretty_hash::fmt as pretty_fmt;
use random_access_disk::RandomAccessDisk;
use random_access_memory::RandomAccessMemory;
use random_access_storage::RandomAccess;
use tree_index::TreeIndex;

use std::borrow::Borrow;
use std::cmp;
use std::fmt::{self, Debug, Display};
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

/// Append-only log structure.
#[derive(Debug)]
pub struct Feed<T>
where
    T: RandomAccess<Error = Box<dyn std::error::Error + Send + Sync>> + Debug,
{
    /// Merkle tree instance.
    pub(crate) merkle: Merkle,
    pub(crate) public_key: PublicKey,
    pub(crate) secret_key: Option<SecretKey>,
    pub(crate) storage: Storage<T>,
    /// Total length of data stored.
    pub(crate) byte_length: u64,
    /// TODO: description. Length of... roots?
    pub(crate) length: u64,
    /// Bitfield to keep track of which data we own.
    pub(crate) bitfield: Bitfield,
    pub(crate) tree: TreeIndex,
    pub(crate) peers: Vec<Peer>,
}

impl<T> Feed<T>
where
    T: RandomAccess<Error = Box<dyn std::error::Error + Send + Sync>> + Debug + Send,
{
    /// Create a new instance with a custom storage backend.
    pub async fn with_storage(mut storage: crate::storage::Storage<T>) -> Result<Self> {
        match storage.read_partial_keypair().await {
            Some(partial_keypair) => {
                let builder = FeedBuilder::new(partial_keypair.public, storage);

                // return early without secret key
                if partial_keypair.secret.is_none() {
                    return Ok(builder.build()?);
                }

                builder.secret_key(partial_keypair.secret.unwrap()).build()
            }
            None => {
                // we have no keys, generate a pair and save them to the storage
                let keypair = generate_keypair();
                storage.write_public_key(&keypair.public).await?;
                storage.write_secret_key(&keypair.secret).await?;

                FeedBuilder::new(keypair.public, storage)
                    .secret_key(keypair.secret)
                    .build()
            }
        }
    }

    /// Starts a `FeedBuilder` with the provided `Keypair` and `Storage`.
    pub fn builder(public_key: PublicKey, storage: Storage<T>) -> FeedBuilder<T> {
        FeedBuilder::new(public_key, storage)
    }

    /// Get the amount of entries in the feed.
    #[inline]
    pub fn len(&self) -> u64 {
        self.length
    }

    /// Check if the length is 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the total amount of bytes stored in the feed.
    #[inline]
    pub fn byte_len(&self) -> u64 {
        self.byte_length
    }

    /// Append data into the log.
    #[inline]
    pub async fn append(&mut self, data: &[u8]) -> Result<()> {
        let key = match &self.secret_key {
            Some(key) => key,
            None => bail!("no secret key, cannot append."),
        };
        self.merkle.next(data);
        let mut offset = 0;

        self.storage
            .write_data(self.byte_length + offset, &data)
            .await?;
        offset += data.len() as u64;

        let hash = Hash::from_roots(self.merkle.roots());
        let index = self.length;
        let message = hash_with_length_as_bytes(hash, index + 1);
        let signature = sign(&self.public_key, key, &message);
        self.storage.put_signature(index, signature).await?;

        for node in self.merkle.nodes() {
            self.storage.put_node(node).await?;
        }

        self.byte_length += offset;

        self.bitfield.set(self.length, true);
        self.tree.set(tree_index(self.length));
        self.length += 1;

        Ok(())
    }

    /// Get the block of data at the tip of the feed. This will be the most
    /// recently appended block.
    #[inline]
    pub async fn head(&mut self) -> Result<Option<Vec<u8>>> {
        match self.len() {
            0 => Ok(None),
            len => self.get(len - 1).await,
        }
    }

    /// Return `true` if a data block is available locally.
    #[inline]
    pub fn has(&mut self, index: u64) -> bool {
        self.bitfield.get(index)
    }

    /// Return `true` if all data blocks within a range are available locally.
    #[inline]
    pub fn has_all(&mut self, range: ::std::ops::Range<u64>) -> bool {
        let total = range.clone().count();
        total == self.bitfield.total_with_range(range) as usize
    }

    /// Get the total amount of chunks downloaded.
    #[inline]
    pub fn downloaded(&mut self, range: ::std::ops::Range<u64>) -> u8 {
        self.bitfield.total_with_range(range)
    }

    /// Retrieve data from the log.
    #[inline]
    pub async fn get(&mut self, index: u64) -> Result<Option<Vec<u8>>> {
        if !self.bitfield.get(index) {
            // NOTE: Do (network) lookup here once we have network code.
            return Ok(None);
        }
        Ok(Some(self.storage.get_data(index).await?))
    }

    /// Return the Nodes which prove the correctness for the Node at index.
    #[inline]
    pub async fn proof(&mut self, index: u64, include_hash: bool) -> Result<Proof> {
        self.proof_with_digest(index, 0, include_hash).await
    }

    /// Return the Nodes which prove the correctness for the Node at index with a
    /// digest.
    pub async fn proof_with_digest(
        &mut self,
        index: u64,
        digest: u64,
        include_hash: bool,
    ) -> Result<Proof> {
        let mut remote_tree = TreeIndex::default();
        let mut nodes = vec![];

        let proof = self.tree.proof_with_digest(
            tree_index(index),
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
            match self.storage.get_signature(sig_index).await {
                Ok(sig) => Some(sig),
                Err(_) => None,
            }
        };

        let mut nodes = Vec::with_capacity(proof.nodes().len());
        for index in proof.nodes() {
            let node = self.storage.get_node(*index).await?;
            nodes.push(node);
        }

        Ok(Proof {
            nodes,
            signature,
            index,
        })
    }

    /// Compute the digest for the index.
    pub fn digest(&mut self, index: u64) -> u64 {
        self.tree.digest(tree_index(index))
    }

    /// Insert data into the tree at `index`. Verifies the `proof` when inserting
    /// to make sure data is correct. Useful when replicating data from a remote
    /// host.
    pub async fn put(&mut self, index: u64, data: Option<&[u8]>, mut proof: Proof) -> Result<()> {
        let mut next = tree_index(index);
        let mut trusted: Option<u64> = None;
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
            let node = self.storage.get_node(index).await?;
            missing_nodes.push(node);
        }

        let mut trusted_node = None;
        if let Some(index) = trusted {
            let node = self.storage.get_node(index).await?;
            trusted_node = Some(node);
        }

        let mut visited = vec![];
        let mut top = match data {
            Some(data) => Node::new(
                tree_index(index),
                Hash::from_leaf(&data).as_bytes().to_owned(),
                data.len() as u64,
            ),
            None => proof.nodes.remove(0),
        };

        // check if we already have the hash for this node
        if verify_node(&trusted_node, &top) {
            self.write(index, data, &visited, None).await?;
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
                let nodes = self.verify_roots(&top, &mut proof).await?;
                visited.extend_from_slice(&nodes);
                self.write(index, data, &visited, proof.signature).await?;
                return Ok(());
            }

            visited.push(top.clone());
            let hash = Hash::from_hashes(&top, &node);
            let len = top.len() + node.len();
            top = Node::new(flat::parent(top.index), hash.as_bytes().into(), len);

            if verify_node(&trusted_node, &top) {
                self.write(index, data, &visited, None).await?;
                return Ok(());
            }
        }

        fn verify_node(trusted: &Option<Node>, node: &Node) -> bool {
            match trusted {
                None => false,
                Some(trusted) => trusted.index == node.index && trusted.hash == node.hash,
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
    async fn write(
        &mut self,
        index: u64,
        data: Option<&[u8]>,
        nodes: &[Node],
        sig: Option<Signature>,
    ) -> Result<()> {
        for node in nodes {
            self.storage.put_node(node).await?;
        }

        if let Some(data) = data {
            self.storage.put_data(index, data, &nodes).await?;
        }

        if let Some(sig) = sig {
            let sig = sig.borrow();
            self.storage.put_signature(index, sig).await?;
        }

        for node in nodes {
            self.tree.set(node.index);
        }

        self.tree.set(tree_index(index));

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
    pub async fn signature(&mut self, index: u64) -> Result<Signature> {
        ensure!(
            index < self.length,
            format!("No signature found for index {}", index)
        );
        self.storage.next_signature(index).await
    }

    /// Verify the entire feed. Checks a signature against the signature of all
    /// root nodes combined.
    pub async fn verify(&mut self, index: u64, signature: &Signature) -> Result<()> {
        let roots = self.root_hashes(index).await?;
        let roots: Vec<_> = roots.into_iter().map(Arc::new).collect();

        let hash = Hash::from_roots(&roots);
        let message = hash_with_length_as_bytes(hash, index + 1);

        verify(&self.public_key, &message, Some(signature))?;
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
    pub async fn root_hashes(&mut self, index: u64) -> Result<Vec<Node>> {
        ensure!(
            index <= self.length,
            format!("Root index bounds exceeded {} > {}", index, self.length)
        );
        let roots_index = tree_index(index) + 2;
        let mut indexes = vec![];
        flat::full_roots(roots_index, &mut indexes);

        let mut roots = Vec::with_capacity(indexes.len());
        for index in indexes {
            let node = self.storage.get_node(index).await?;
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

    async fn verify_roots(&mut self, top: &Node, proof: &mut Proof) -> Result<Vec<Node>> {
        let last_node = if !proof.nodes.is_empty() {
            proof.nodes[proof.nodes.len() - 1].index
        } else {
            top.index
        };

        let verified_by = cmp::max(flat::right_span(top.index), flat::right_span(last_node)) + 2;

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
                let node = self.storage.get_node(index).await?;
                roots.push(node);
            } else {
                bail!("<hypercore>: Missing tree roots needed for verify");
            }
        }

        let checksum = Hash::from_roots(&roots);
        let length = verified_by / 2;
        let message = hash_with_length_as_bytes(checksum, length);
        verify(&self.public_key, &message, proof.signature())?;

        // Update the length if we grew the feed.
        let len = verified_by / 2;
        if len > self.len() {
            self.length = len;
            self.byte_length = roots.iter().fold(0, |acc, root| acc + root.index)
            // TODO: emit('append')
        }

        Ok(extra_nodes)
    }

    /// Audit all data in the feed. Checks that all current data matches
    /// the hashes in the merkle tree, and clears the bitfield if not.
    /// The tuple returns is (valid_blocks, invalid_blocks)
    pub async fn audit(&mut self) -> Result<Audit> {
        let mut valid_blocks = 0;
        let mut invalid_blocks = 0;
        for index in 0..self.length {
            if self.bitfield.get(index) {
                let node = self.storage.get_node(2 * index).await?;
                let data = self.storage.get_data(index).await?;
                let data_hash = Hash::from_leaf(&data);
                if node.hash == data_hash.as_bytes() {
                    valid_blocks += 1;
                } else {
                    invalid_blocks += 1;
                    self.bitfield.set(index, false);
                }
            }
        }
        Ok(Audit {
            valid_blocks,
            invalid_blocks,
        })
    }

    /// Expose the bitfield attribute to use on during download
    pub fn bitfield(&self) -> &Bitfield {
        &self.bitfield
    }

    /// (unimplemented) Provide a range of data to download.
    pub fn download(&mut self, _range: Range<u64>) -> Result<()> {
        unimplemented!();
    }

    /// (unimplemented) Provide a range of data to remove from the local storage.
    pub fn undownload(&mut self, _range: Range<u64>) -> Result<()> {
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

impl Feed<RandomAccessDisk> {
    /// Create a new instance that persists to disk at the location of `dir`.
    // TODO: Ensure that dir is always a directory.
    // NOTE: Should we `mkdirp` here?
    // NOTE: Should we call these `data.bitfield` / `data.tree`?
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let dir = path.as_ref().to_owned();
        let storage = Storage::new_disk(&dir).await?;
        Self::with_storage(storage).await
    }
}

/// Create a new instance with an in-memory storage backend.
///
/// ## Panics
/// Can panic if constructing the in-memory store fails, which is highly
/// unlikely.
impl Default for Feed<RandomAccessMemory> {
    fn default() -> Self {
        async_std::task::block_on(async {
            let storage = Storage::new_memory().await.unwrap();
            Self::with_storage(storage).await.unwrap()
        })
    }
}

impl<T: RandomAccess<Error = Box<dyn std::error::Error + Send + Sync>> + Debug + Send> Display
    for Feed<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// Convert the index to the index in the tree.
#[inline]
fn tree_index(index: u64) -> u64 {
    2 * index
}

/// Extend a hash with a big-endian encoded length.
fn hash_with_length_as_bytes(hash: Hash, length: u64) -> Vec<u8> {
    [hash.as_bytes(), &length.to_be_bytes()].concat().to_vec()
}
