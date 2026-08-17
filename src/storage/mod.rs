//! Save data to a desired storage backend.

use futures::future::FutureExt;
#[cfg(not(target_arch = "wasm32"))]
use random_access_disk::RandomAccessDisk;
use random_access_memory::RandomAccessMemory;
use random_access_storage::{BoxFuture, RandomAccess, RandomAccessError};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::{fmt::Debug, sync::Arc};
use tracing::instrument;

use crate::{
    HypercoreError,
    common::{Store, StoreInfo, StoreInfoInstruction, StoreInfoType},
};

/// Supertrait for Storage
pub trait StorageTraits: RandomAccess + Debug + Send + Sync {}
impl<T: RandomAccess + Debug + Send + Sync> StorageTraits for T {}

/// Save data to a desired storage backend.
#[derive(Debug, Clone)]
pub struct Storage {
    tree: Arc<dyn StorageTraits>,
    data: Arc<dyn StorageTraits>,
    bitfield: Arc<dyn StorageTraits>,
    oplog: Arc<dyn StorageTraits>,
}

impl Storage {
    /// Create a new instance. Takes a callback to create new storage instances and overwrite flag.
    pub async fn open<Cb>(create: Cb, overwrite: bool) -> Result<Self, HypercoreError>
    where
        Cb: Fn(
            Store,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<Arc<dyn StorageTraits>, RandomAccessError>>
                    + Send,
            >,
        >,
    {
        let tree: Arc<dyn StorageTraits> = create(Store::Tree).await?;
        let data: Arc<dyn StorageTraits> = create(Store::Data).await?;
        let bitfield: Arc<dyn StorageTraits> = create(Store::Bitfield).await?;
        let oplog: Arc<dyn StorageTraits> = create(Store::Oplog).await?;

        if overwrite {
            if tree.len() > 0 {
                tree.truncate(0).await?;
            }
            if data.len() > 0 {
                data.truncate(0).await?;
            }
            if bitfield.len() > 0 {
                bitfield.truncate(0).await?;
            }
            if oplog.len() > 0 {
                oplog.truncate(0).await?;
            }
        }

        Ok(Self {
            tree,
            data,
            bitfield,
            oplog,
        })
    }

    /// Read info from store based on given instruction.
    pub(crate) fn read_info(
        &self,
        info_instruction: StoreInfoInstruction,
    ) -> BoxFuture<Result<StoreInfo, HypercoreError>> {
        let fut = self.read_infos_to_vec(vec![info_instruction]);
        Box::pin(async move {
            Ok(fut
                .await?
                .pop()
                .expect("Should have gotten one info with one instruction"))
        })
    }

    /// Read infos from stores based on given instructions
    pub(crate) fn read_infos(
        &self,
        info_instructions: Vec<StoreInfoInstruction>,
    ) -> BoxFuture<Result<Box<[StoreInfo]>, HypercoreError>> {
        let fut = self.read_infos_to_vec(info_instructions);
        Box::pin(async move { Ok(fut.await?.into_boxed_slice()) })
    }

    /// Reads infos but retains them as a Vec
    pub(crate) fn read_infos_to_vec(
        &self,
        info_instructions: Vec<StoreInfoInstruction>,
    ) -> BoxFuture<Result<Vec<StoreInfo>, HypercoreError>> {
        let storage = self.clone();
        let instructions = info_instructions; // TODO rm
        Box::pin(async move {
            if instructions.is_empty() {
                return Ok(vec![]);
            }
            let mut current_store: Store = instructions[0].store.clone();
            let mut ra: Arc<dyn StorageTraits> = storage.get_random_access(&current_store).clone();
            let mut infos: Vec<StoreInfo> = Vec::with_capacity(instructions.len());
            for instruction in instructions.iter() {
                if instruction.store != current_store {
                    current_store = instruction.store.clone();
                    ra = storage.get_random_access(&current_store).clone();
                }
                match instruction.info_type {
                    StoreInfoType::Content => {
                        let read_length = match instruction.length {
                            Some(length) => length,
                            None => ra.len(),
                        };
                        let read_result = ra.read(instruction.index, read_length).await;
                        let info: StoreInfo = match read_result {
                            Ok(buf) => Ok(StoreInfo::new_content(
                                instruction.store.clone(),
                                instruction.index,
                                &buf,
                            )),
                            Err(RandomAccessError::OutOfBounds { length, .. }) => {
                                if instruction.allow_miss {
                                    Ok(StoreInfo::new_content_miss(
                                        instruction.store.clone(),
                                        instruction.index,
                                    ))
                                } else {
                                    Err(HypercoreError::InvalidOperation {
                                        context: format!(
                                            "Could not read from store {}, index {} / length {} is out of bounds for store length {}",
                                            current_store, instruction.index, read_length, length
                                        ),
                                    })
                                }
                            }
                            Err(e) => Err(HypercoreError::from(e)),
                        }?;
                        infos.push(info);
                    }
                    StoreInfoType::Size => {
                        let length = ra.len();
                        infos.push(StoreInfo::new_size(
                            instruction.store.clone(),
                            instruction.index,
                            length - instruction.index,
                        ));
                    }
                }
            }
            Ok(infos)
        })
    }

    /// Flush info to storage.
    pub(crate) fn flush_info(&self, info: StoreInfo) -> BoxFuture<Result<(), HypercoreError>> {
        self.flush_infos(vec![info])
    }

    /// Flush infos to storage
    pub(crate) fn flush_infos(
        &self,
        infos: Vec<StoreInfo>,
    ) -> BoxFuture<Result<(), HypercoreError>> {
        let storage = self.clone();
        Box::pin(async move {
            if infos.is_empty() {
                return Ok(());
            }
            let mut current_store: Store = infos[0].store.clone();
            let mut ra: Arc<dyn StorageTraits> = storage.get_random_access(&current_store).clone();
            for info in infos.iter() {
                if info.store != current_store {
                    current_store = info.store.clone();
                    ra = storage.get_random_access(&current_store).clone();
                }
                match info.info_type {
                    StoreInfoType::Content => {
                        if !info.miss {
                            if let Some(data) = &info.data {
                                ra.write(info.index, data).await?;
                            }
                        } else {
                            ra.del(
                                info.index,
                                info.length.expect("When deleting, length must be given"),
                            )
                            .await?;
                        }
                    }
                    StoreInfoType::Size => {
                        if info.miss {
                            ra.truncate(info.index).await?;
                        } else {
                            panic!("Flushing a size that isn't miss, is not supported");
                        }
                    }
                }
            }
            Ok(())
        })
    }

    fn get_random_access(&self, store: &Store) -> &Arc<dyn StorageTraits> {
        match store {
            Store::Tree => &self.tree,
            Store::Data => &self.data,
            Store::Bitfield => &self.bitfield,
            Store::Oplog => &self.oplog,
        }
    }

    /// New storage backed by a `RandomAccessMemory` instance.
    #[instrument(err)]
    pub async fn new_memory() -> Result<Self, HypercoreError> {
        let create = |_| {
            async { Ok(Arc::new(RandomAccessMemory::default()) as Arc<dyn StorageTraits>) }.boxed()
        };
        Self::open(create, false).await
    }

    /// New storage backed by a `RandomAccessDisk` instance.
    #[cfg(not(target_arch = "wasm32"))]
    #[instrument(err)]
    pub async fn new_disk(dir: &PathBuf, overwrite: bool) -> Result<Self, HypercoreError> {
        let storage = |store: Store| {
            let dir = dir.clone();
            async move {
                let name = match store {
                    Store::Tree => "tree",
                    Store::Data => "data",
                    Store::Bitfield => "bitfield",
                    Store::Oplog => "oplog",
                };
                Ok(
                    Arc::new(RandomAccessDisk::open(dir.as_path().join(name)).await?)
                        as Arc<dyn StorageTraits>,
                )
            }
            .boxed()
        };
        Self::open(storage, overwrite).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::common::{StoreInfo, StoreInfoInstruction};

    #[tokio::test]
    async fn test_futures_are_owned() -> Result<(), Box<dyn std::error::Error>> {
        let fut = {
            let storage = Storage::new_memory().await?;

            let data = b"hello hypercore";

            // Write to tree store
            storage.flush_info(StoreInfo::new_content(Store::Tree, 0, data))
        };
        fut.await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_storage() -> Result<(), Box<dyn std::error::Error>> {
        let storage = Storage::new_memory().await?;

        let data = b"hello hypercore";

        // Write to tree store
        storage
            .flush_info(StoreInfo::new_content(Store::Tree, 0, data))
            .await?;

        // Read it back
        let info = storage
            .read_info(StoreInfoInstruction::new_content(
                Store::Tree,
                0,
                data.len() as u64,
            ))
            .await?;

        assert_eq!(info.data.as_deref(), Some(data.as_slice()));

        // Write to two different stores, read back together
        storage
            .flush_infos(vec![
                StoreInfo::new_content(Store::Data, 0, b"block0"),
                StoreInfo::new_content(Store::Bitfield, 0, b"bits"),
            ])
            .await?;

        let infos = storage
            .read_infos(vec![
                StoreInfoInstruction::new_content(Store::Data, 0, 6),
                StoreInfoInstruction::new_content(Store::Bitfield, 0, 4),
            ])
            .await?;

        assert_eq!(infos[0].data.as_deref(), Some(b"block0".as_slice()));
        assert_eq!(infos[1].data.as_deref(), Some(b"bits".as_slice()));

        Ok(())
    }
}
