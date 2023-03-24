pub mod common;

use anyhow::Result;
use common::get_test_key_pair;
use hypercore::{Builder, Storage};
use test_log::test;

#[test(async_std::test)]
async fn hypercore_new() -> Result<()> {
    let storage = Storage::new_memory().await?;
    let _hypercore = Builder::new(storage).build();
    Ok(())
}

#[test(async_std::test)]
async fn hypercore_new_with_key_pair() -> Result<()> {
    let storage = Storage::new_memory().await?;
    let key_pair = get_test_key_pair();
    let _hypercore = Builder::new(storage).set_key_pair(key_pair).build().await?;
    Ok(())
}

#[test(async_std::test)]
async fn hypercore_open_with_key_pair_error() -> Result<()> {
    let storage = Storage::new_memory().await?;
    let key_pair = get_test_key_pair();
    assert!(Builder::new(storage)
        .set_key_pair(key_pair)
        .set_open(true)
        .build()
        .await
        .is_err());
    Ok(())
}
