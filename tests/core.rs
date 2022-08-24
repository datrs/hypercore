mod common;

use anyhow::Result;
use common::get_test_key_pair;
#[cfg(feature = "v10")]
use hypercore::{Hypercore, Storage};

#[async_std::test]
#[cfg(feature = "v10")]
async fn hypercore_new() -> Result<()> {
    let storage = Storage::new_memory().await?;
    let _hypercore = Hypercore::new(storage).await?;
    Ok(())
}

#[async_std::test]
#[cfg(feature = "v10")]
async fn hypercore_new_with_key_pair() -> Result<()> {
    let storage = Storage::new_memory().await?;
    let key_pair = get_test_key_pair();
    let _hypercore = Hypercore::new_with_key_pair(storage, key_pair).await?;
    Ok(())
}
