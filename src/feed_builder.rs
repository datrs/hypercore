use ed25519_dalek::{PublicKey, SecretKey};

use crate::bitfield::Bitfield;
use crate::crypto::Merkle;
use crate::storage::BoxStorage;
use std::fmt::Debug;
use tree_index::TreeIndex;

use crate::Feed;
use anyhow::Result;

/// Construct a new `Feed` instance.
// TODO: make this an actual builder pattern.
// https://deterministic.space/elegant-apis-in-rust.html#builder-pattern
#[derive(Debug)]
pub struct FeedBuilder {
    storage: BoxStorage,
    public_key: PublicKey,
    secret_key: Option<SecretKey>,
}

impl FeedBuilder {
    /// Create a new instance.
    #[inline]
    pub fn new(public_key: PublicKey, storage: BoxStorage) -> Self {
        Self {
            storage,
            public_key,
            secret_key: None,
        }
    }

    /// Set the secret key.
    pub fn secret_key(mut self, secret_key: SecretKey) -> Self {
        self.secret_key = Some(secret_key);
        self
    }

    /// Finalize the builder.
    #[inline]
    pub async fn build(mut self) -> Result<Feed> {
        let (bitfield, tree) = if let Ok(bitfield) = self.storage.read_bitfield().await {
            Bitfield::from_slice(&bitfield)
        } else {
            Bitfield::new()
        };
        use crate::storage::Node;

        let mut tree = TreeIndex::new(tree);
        let mut roots = vec![];
        flat_tree::full_roots(tree.blocks() * 2, &mut roots);
        let mut result: Vec<Option<Node>> = vec![None; roots.len()];

        for i in 0..roots.len() {
            let node = self.storage.get_node(roots[i] as u64).await?;
            let idx = roots
                .iter()
                .position(|&x| x == node.index)
                .ok_or_else(|| anyhow::anyhow!("Couldnt find idx of node"))?;
            result[idx] = Some(node);
        }

        let roots = result
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| anyhow::anyhow!("Roots contains undefined nodes"))?;

        let byte_length = roots.iter().fold(0, |acc, node| acc + node.length);

        Ok(Feed {
            merkle: Merkle::from_nodes(roots),
            byte_length,
            length: tree.blocks(),
            bitfield,
            tree,
            public_key: self.public_key,
            secret_key: self.secret_key,
            storage: self.storage,
            peers: vec![],
        })
    }
}
