//! Save data to a desired storage backend.

use futures::future::FutureExt;
#[cfg(not(target_arch = "wasm32"))]
use random_access_disk::RandomAccessDisk;
use random_access_memory::RandomAccessMemory;
use random_access_storage::{RandomAccess, RandomAccessError};
use std::fmt::Debug;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use tracing::instrument;

use crate::{
    common::{Store, StoreInfo, StoreInfoInstruction, StoreInfoType},
    HypercoreError,
};

/// Save data to a desired storage backend.
#[derive(Debug)]
pub struct Storage<T>
where
    T: RandomAccess + Debug,
{
    tree: T,
    data: T,
    bitfield: T,
    oplog: T,
}

pub(crate) fn map_random_access_err(err: RandomAccessError) -> HypercoreError {
    match err {
        RandomAccessError::IO {
            return_code,
            context,
            source,
        } => HypercoreError::IO {
            context: Some(format!(
                "RandomAccess IO error. Context: {context:?}, return_code: {return_code:?}",
            )),
            source,
        },
        RandomAccessError::OutOfBounds {
            offset,
            end,
            length,
        } => HypercoreError::InvalidOperation {
            context: format!(
                "RandomAccess out of bounds. Offset: {offset}, end: {end:?}, length: {length}",
            ),
        },
    }
}

impl<T> Storage<T>
where
    T: RandomAccess + Debug + Send,
{
    /// Create a new instance. Takes a callback to create new storage instances and overwrite flag.
    pub async fn open<Cb>(create: Cb, overwrite: bool) -> Result<Self, HypercoreError>
    where
        Cb: Fn(
            Store,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<T, RandomAccessError>> + Send>,
        >,
    {
        let mut tree = create(Store::Tree).await.map_err(map_random_access_err)?;
        let mut data = create(Store::Data).await.map_err(map_random_access_err)?;
        let mut bitfield = create(Store::Bitfield)
            .await
            .map_err(map_random_access_err)?;
        let mut oplog = create(Store::Oplog).await.map_err(map_random_access_err)?;

        if overwrite {
            if tree.len().await.map_err(map_random_access_err)? > 0 {
                tree.truncate(0).await.map_err(map_random_access_err)?;
            }
            if data.len().await.map_err(map_random_access_err)? > 0 {
                data.truncate(0).await.map_err(map_random_access_err)?;
            }
            if bitfield.len().await.map_err(map_random_access_err)? > 0 {
                bitfield.truncate(0).await.map_err(map_random_access_err)?;
            }
            if oplog.len().await.map_err(map_random_access_err)? > 0 {
                oplog.truncate(0).await.map_err(map_random_access_err)?;
            }
        }

        let instance = Self {
            tree,
            data,
            bitfield,
            oplog,
        };

        Ok(instance)
    }

    /// Read info from store based on given instruction. Convenience method to `read_infos`.
    pub(crate) async fn read_info(
        &mut self,
        info_instruction: StoreInfoInstruction,
    ) -> Result<StoreInfo, HypercoreError> {
        let mut infos = self.read_infos_to_vec(&[info_instruction]).await?;
        Ok(infos
            .pop()
            .expect("Should have gotten one info with one instruction"))
    }

    /// Read infos from stores based on given instructions
    pub(crate) async fn read_infos(
        &mut self,
        info_instructions: &[StoreInfoInstruction],
    ) -> Result<Box<[StoreInfo]>, HypercoreError> {
        let infos = self.read_infos_to_vec(info_instructions).await?;
        Ok(infos.into_boxed_slice())
    }

    /// Reads infos but retains them as a Vec
    pub(crate) async fn read_infos_to_vec(
        &mut self,
        info_instructions: &[StoreInfoInstruction],
    ) -> Result<Vec<StoreInfo>, HypercoreError> {
        if info_instructions.is_empty() {
            return Ok(vec![]);
        }
        let mut current_store: Store = info_instructions[0].store.clone();
        let mut storage = self.get_random_access(&current_store);
        let mut infos: Vec<StoreInfo> = Vec::with_capacity(info_instructions.len());
        for instruction in info_instructions.iter() {
            if instruction.store != current_store {
                current_store = instruction.store.clone();
                storage = self.get_random_access(&current_store);
            }
            match instruction.info_type {
                StoreInfoType::Content => {
                    let read_length = match instruction.length {
                        Some(length) => length,
                        None => storage.len().await.map_err(map_random_access_err)?,
                    };
                    let read_result = storage.read(instruction.index, read_length).await;
                    let info: StoreInfo = match read_result {
                        Ok(buf) => Ok(StoreInfo::new_content(
                            instruction.store.clone(),
                            instruction.index,
                            &buf,
                        )),
                        Err(RandomAccessError::OutOfBounds {
                            offset: _,
                            end: _,
                            length,
                        }) => {
                            if instruction.allow_miss {
                                Ok(StoreInfo::new_content_miss(
                                    instruction.store.clone(),
                                    instruction.index,
                                ))
                            } else {
                                Err(HypercoreError::InvalidOperation {
                                    context: format!(
                                        "Could not read from store {}, index {} / length {} is out of bounds for store length {}",
                                        instruction.index,
                                        read_length,
                                        current_store,
                                        length
                                    ),
                                })
                            }
                        }
                        Err(e) => Err(map_random_access_err(e)),
                    }?;
                    infos.push(info);
                }
                StoreInfoType::Size => {
                    let length = storage.len().await.map_err(map_random_access_err)?;
                    infos.push(StoreInfo::new_size(
                        instruction.store.clone(),
                        instruction.index,
                        length - instruction.index,
                    ));
                }
            }
        }
        Ok(infos)
    }

    /// Flush info to storage. Convenience method to `flush_infos`.
    pub(crate) async fn flush_info(&mut self, slice: StoreInfo) -> Result<(), HypercoreError> {
        self.flush_infos(&[slice]).await
    }

    /// Flush infos to storage
    pub(crate) async fn flush_infos(&mut self, infos: &[StoreInfo]) -> Result<(), HypercoreError> {
        if infos.is_empty() {
            return Ok(());
        }
        let mut current_store: Store = infos[0].store.clone();
        let mut storage = self.get_random_access(&current_store);
        for info in infos.iter() {
            if info.store != current_store {
                current_store = info.store.clone();
                storage = self.get_random_access(&current_store);
            }
            match info.info_type {
                StoreInfoType::Content => {
                    if !info.miss {
                        if let Some(data) = &info.data {
                            storage
                                .write(info.index, data)
                                .await
                                .map_err(map_random_access_err)?;
                        }
                    } else {
                        storage
                            .del(
                                info.index,
                                info.length.expect("When deleting, length must be given"),
                            )
                            .await
                            .map_err(map_random_access_err)?;
                    }
                }
                StoreInfoType::Size => {
                    if info.miss {
                        storage
                            .truncate(info.index)
                            .await
                            .map_err(map_random_access_err)?;
                    } else {
                        panic!("Flushing a size that isn't miss, is not supported");
                    }
                }
            }
        }
        Ok(())
    }

    fn get_random_access(&mut self, store: &Store) -> &mut T {
        match store {
            Store::Tree => &mut self.tree,
            Store::Data => &mut self.data,
            Store::Bitfield => &mut self.bitfield,
            Store::Oplog => &mut self.oplog,
        }
    }
}

impl Storage<RandomAccessMemory> {
    /// New storage backed by a `RandomAccessMemory` instance.
    #[instrument(err)]
    pub async fn new_memory() -> Result<Self, HypercoreError> {
        let create = |_| async { Ok(RandomAccessMemory::default()) }.boxed();
        // No reason to overwrite, as this is a new memory segment
        Self::open(create, false).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Storage<RandomAccessDisk> {
    /// New storage backed by a `RandomAccessDisk` instance.
    #[instrument(err)]
    pub async fn new_disk(dir: &PathBuf, overwrite: bool) -> Result<Self, HypercoreError> {
        let storage = |store: Store| {
            let name = match store {
                Store::Tree => "tree",
                Store::Data => "data",
                Store::Bitfield => "bitfield",
                Store::Oplog => "oplog",
            };
            RandomAccessDisk::open(dir.as_path().join(name)).boxed()
        };
        Self::open(storage, overwrite).await
    }
}
