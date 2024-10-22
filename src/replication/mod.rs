//! External interface for replication
pub mod events;

use crate::{
    AppendOutcome, Hypercore, HypercoreError, Info, PartialKeypair, Proof, RequestBlock,
    RequestSeek, RequestUpgrade,
};

pub use events::Event;

use async_broadcast::Receiver;
use async_lock::Mutex;

use std::future::Future;
use std::sync::Arc;
/// Hypercore that can have multiple owners
#[derive(Debug, Clone)]
pub struct SharedCore(pub Arc<Mutex<Hypercore>>);

impl From<Hypercore> for SharedCore {
    fn from(core: Hypercore) -> Self {
        SharedCore(Arc::new(Mutex::new(core)))
    }
}
impl SharedCore {
    /// Create a shared core from a [`Hypercore`]
    pub fn from_hypercore(core: Hypercore) -> Self {
        SharedCore(Arc::new(Mutex::new(core)))
    }
}

/// Methods related to just this core's information
pub trait CoreInfo {
    /// Get core info (see: [`Hypercore::info`]
    fn info(&self) -> impl Future<Output = Info> + Send;
    /// Get the key_pair (see: [`Hypercore::key_pair`]
    fn key_pair(&self) -> impl Future<Output = PartialKeypair> + Send;
}

impl CoreInfo for SharedCore {
    fn info(&self) -> impl Future<Output = Info> + Send {
        async move {
            let core = &self.0.lock().await;
            core.info()
        }
    }

    fn key_pair(&self) -> impl Future<Output = PartialKeypair> + Send {
        async move {
            let core = &self.0.lock().await;
            core.key_pair().clone()
        }
    }
}

/// Error for ReplicationMethods trait
#[derive(thiserror::Error, Debug)]
pub enum ReplicationMethodsError {
    /// Error from hypercore
    #[error("Got a hypercore error: [{0}]")]
    HypercoreError(#[from] HypercoreError),
}

/// Methods needed for replication
pub trait ReplicationMethods: CoreInfo + Send {
    /// ref Core::verify_and_apply_proof
    fn verify_and_apply_proof(
        &self,
        proof: &Proof,
    ) -> impl Future<Output = Result<bool, ReplicationMethodsError>> + Send;
    /// ref Core::missing_nodes
    fn missing_nodes(
        &self,
        index: u64,
    ) -> impl Future<Output = Result<u64, ReplicationMethodsError>> + Send;
    /// ref Core::create_proof
    fn create_proof(
        &self,
        block: Option<RequestBlock>,
        hash: Option<RequestBlock>,
        seek: Option<RequestSeek>,
        upgrade: Option<RequestUpgrade>,
    ) -> impl Future<Output = Result<Option<Proof>, ReplicationMethodsError>> + Send;
    /// subscribe to core events
    fn event_subscribe(&self) -> impl Future<Output = Receiver<Event>>;
}

impl ReplicationMethods for SharedCore {
    fn verify_and_apply_proof(
        &self,
        proof: &Proof,
    ) -> impl Future<Output = Result<bool, ReplicationMethodsError>> {
        async move {
            let mut core = self.0.lock().await;
            Ok(core.verify_and_apply_proof(proof).await?)
        }
    }

    fn missing_nodes(
        &self,
        index: u64,
    ) -> impl Future<Output = Result<u64, ReplicationMethodsError>> {
        async move {
            let mut core = self.0.lock().await;
            Ok(core.missing_nodes(index).await?)
        }
    }

    fn create_proof(
        &self,
        block: Option<RequestBlock>,
        hash: Option<RequestBlock>,
        seek: Option<RequestSeek>,
        upgrade: Option<RequestUpgrade>,
    ) -> impl Future<Output = Result<Option<Proof>, ReplicationMethodsError>> {
        async move {
            let mut core = self.0.lock().await;
            Ok(core.create_proof(block, hash, seek, upgrade).await?)
        }
    }

    fn event_subscribe(&self) -> impl Future<Output = Receiver<Event>> {
        async move { self.0.lock().await.event_subscribe() }
    }
}

/// Error for ReplicationMethods trait
#[derive(thiserror::Error, Debug)]
pub enum CoreMethodsError {
    /// Error from hypercore
    #[error("Got a hypercore error [{0}]")]
    HypercoreError(#[from] HypercoreError),
}

/// Trait for things that consume [`crate::Hypercore`] can instead use this trait
/// so they can use all Hypercore-like things such as [`SharedCore`].
pub trait CoreMethods: CoreInfo {
    /// Check if the core has the block at the given index locally
    fn has(&self, index: u64) -> impl Future<Output = bool> + Send;

    /// get a block
    fn get(
        &self,
        index: u64,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, CoreMethodsError>> + Send;

    /// Append data to the core
    fn append(
        &self,
        data: &[u8],
    ) -> impl Future<Output = Result<AppendOutcome, CoreMethodsError>> + Send;

    /// Append a batch of data to the core
    fn append_batch<A: AsRef<[u8]>, B: AsRef<[A]> + Send>(
        &self,
        batch: B,
    ) -> impl Future<Output = Result<AppendOutcome, CoreMethodsError>> + Send;
}

impl CoreMethods for SharedCore {
    fn has(&self, index: u64) -> impl Future<Output = bool> + Send {
        async move {
            let core = self.0.lock().await;
            core.has(index)
        }
    }
    fn get(
        &self,
        index: u64,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, CoreMethodsError>> + Send {
        async move {
            let mut core = self.0.lock().await;
            Ok(core.get(index).await?)
        }
    }

    fn append(
        &self,
        data: &[u8],
    ) -> impl Future<Output = Result<AppendOutcome, CoreMethodsError>> + Send {
        async move {
            let mut core = self.0.lock().await;
            Ok(core.append(data).await?)
        }
    }

    fn append_batch<A: AsRef<[u8]>, B: AsRef<[A]> + Send>(
        &self,
        batch: B,
    ) -> impl Future<Output = Result<AppendOutcome, CoreMethodsError>> + Send {
        async move {
            let mut core = self.0.lock().await;
            Ok(core.append_batch(batch).await?)
        }
    }
}

#[cfg(test)]
mod tests {
    use events::{Get, Have};

    use super::*;

    #[async_std::test]
    async fn shared_core_methods() -> Result<(), CoreMethodsError> {
        let core = crate::core::tests::create_hypercore_with_data(0).await?;
        let core = SharedCore::from(core);

        let info = core.info().await;
        assert_eq!(
            info,
            crate::core::Info {
                length: 0,
                byte_length: 0,
                contiguous_length: 0,
                fork: 0,
                writeable: true,
            }
        );

        // key_pair is random, nothing to test here
        let _kp = core.key_pair().await;

        assert_eq!(core.has(0).await, false);
        assert_eq!(core.get(0).await?, None);
        let res = core.append(b"foo").await?;
        assert_eq!(
            res,
            AppendOutcome {
                length: 1,
                byte_length: 3
            }
        );
        assert_eq!(core.has(0).await, true);
        assert_eq!(core.get(0).await?, Some(b"foo".into()));
        let res = core.append_batch([b"hello", b"world"]).await?;
        assert_eq!(
            res,
            AppendOutcome {
                length: 3,
                byte_length: 13
            }
        );
        assert_eq!(core.has(2).await, true);
        assert_eq!(core.get(2).await?, Some(b"world".into()));
        Ok(())
    }

    #[async_std::test]
    async fn test_events() -> Result<(), CoreMethodsError> {
        let core = crate::core::tests::create_hypercore_with_data(0).await?;
        let core = SharedCore::from(core);

        // Check that appending data emits a DataUpgrade and Have event

        let mut rx = core.event_subscribe().await;
        let handle = async_std::task::spawn(async move {
            let mut out = vec![];
            loop {
                if out.len() == 2 {
                    return (out, rx);
                }
                if let Ok(evt) = rx.recv().await {
                    out.push(evt);
                }
            }
        });
        core.append(b"foo").await?;
        let (res, mut rx) = handle.await;
        assert!(matches!(res[0], Event::DataUpgrade(_)));
        assert!(matches!(
            res[1],
            Event::Have(Have {
                start: 0,
                length: 1,
                drop: false
            })
        ));
        // no messages in queue
        assert!(rx.is_empty());

        // Check that Hypercore::get for missing data emits a Get event

        let handle = async_std::task::spawn(async move {
            let mut out = vec![];
            loop {
                if out.len() == 1 {
                    return (out, rx);
                }
                if let Ok(evt) = rx.recv().await {
                    out.push(evt);
                }
            }
        });
        assert_eq!(core.get(1).await?, None);
        let (res, rx) = handle.await;
        assert!(matches!(
            res[0],
            Event::Get(Get {
                index: 1,
                get_result: _
            })
        ));
        // no messages in queue
        assert!(rx.is_empty());
        Ok(())
    }
}
