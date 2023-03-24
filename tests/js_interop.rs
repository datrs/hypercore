pub mod common;
pub mod js;
use std::{path::Path, sync::Once};

use anyhow::Result;
use common::{create_hypercore_hash, get_test_key_pair};
use hypercore::{Builder, Hypercore, Storage};
use js::{cleanup, install, js_run_step, prepare_test_set};
use random_access_disk::RandomAccessDisk;
use test_log::test;

#[cfg(feature = "async-std")]
use async_std::test as async_test;
#[cfg(feature = "tokio")]
use tokio::test as async_test;

const TEST_SET_JS_FIRST: &str = "jsfirst";
const TEST_SET_RS_FIRST: &str = "rsfirst";

static INIT: Once = Once::new();
fn init() {
    INIT.call_once(|| {
        // run initialization here
        cleanup();
        install();
    });
}

#[test(async_test)]
#[cfg_attr(not(feature = "js_interop_tests"), ignore)]
async fn js_interop_js_first() -> Result<()> {
    init();
    let work_dir = prepare_test_set(TEST_SET_JS_FIRST);
    assert_eq!(create_hypercore_hash(&work_dir), step_0_hash());
    js_run_step(1, TEST_SET_JS_FIRST);
    assert_eq!(create_hypercore_hash(&work_dir), step_1_hash());
    step_2_append_hello_world(&work_dir).await?;
    assert_eq!(create_hypercore_hash(&work_dir), step_2_hash());
    js_run_step(3, TEST_SET_JS_FIRST);
    assert_eq!(create_hypercore_hash(&work_dir), step_3_hash());
    step_4_append_with_flush(&work_dir).await?;
    assert_eq!(create_hypercore_hash(&work_dir), step_4_hash());
    js_run_step(5, TEST_SET_JS_FIRST);
    assert_eq!(create_hypercore_hash(&work_dir), step_5_hash());
    Ok(())
}

#[test(async_test)]
#[cfg_attr(not(feature = "js_interop_tests"), ignore)]
async fn js_interop_rs_first() -> Result<()> {
    init();
    let work_dir = prepare_test_set(TEST_SET_RS_FIRST);
    assert_eq!(create_hypercore_hash(&work_dir), step_0_hash());
    step_1_create(&work_dir).await?;
    assert_eq!(create_hypercore_hash(&work_dir), step_1_hash());
    js_run_step(2, TEST_SET_RS_FIRST);
    assert_eq!(create_hypercore_hash(&work_dir), step_2_hash());
    step_3_read_and_append_unflushed(&work_dir).await?;
    assert_eq!(create_hypercore_hash(&work_dir), step_3_hash());
    js_run_step(4, TEST_SET_RS_FIRST);
    assert_eq!(create_hypercore_hash(&work_dir), step_4_hash());
    step_5_clear_some(&work_dir).await?;
    assert_eq!(create_hypercore_hash(&work_dir), step_5_hash());
    Ok(())
}

async fn step_1_create(work_dir: &str) -> Result<()> {
    create_hypercore(work_dir).await?;
    Ok(())
}

async fn step_2_append_hello_world(work_dir: &str) -> Result<()> {
    let mut hypercore = open_hypercore(work_dir).await?;
    let append_outcome = hypercore.append_batch(&[b"Hello", b"World"]).await?;
    assert_eq!(append_outcome.length, 2);
    assert_eq!(append_outcome.byte_length, 10);
    Ok(())
}

async fn step_3_read_and_append_unflushed(work_dir: &str) -> Result<()> {
    let mut hypercore = open_hypercore(work_dir).await?;
    let hello = hypercore.get(0).await?;
    assert_eq!(hello.unwrap(), b"Hello");
    let world = hypercore.get(1).await?;
    assert_eq!(world.unwrap(), b"World");
    let append_outcome = hypercore.append(b"first").await?;
    assert_eq!(append_outcome.length, 3);
    assert_eq!(append_outcome.byte_length, 15);
    let append_outcome = hypercore.append_batch(&[b"second", b"third"]).await?;
    assert_eq!(append_outcome.length, 5);
    assert_eq!(append_outcome.byte_length, 26);
    let multi_block = &[0x61 as u8; 4096 * 3];
    let append_outcome = hypercore.append(multi_block).await?;
    assert_eq!(append_outcome.length, 6);
    assert_eq!(append_outcome.byte_length, 12314);
    let append_outcome = hypercore.append_batch(&[]).await?;
    assert_eq!(append_outcome.length, 6);
    assert_eq!(append_outcome.byte_length, 12314);
    let first = hypercore.get(2).await?;
    assert_eq!(first.unwrap(), b"first");
    let second = hypercore.get(3).await?;
    assert_eq!(second.unwrap(), b"second");
    let third = hypercore.get(4).await?;
    assert_eq!(third.unwrap(), b"third");
    let multi_block_read = hypercore.get(5).await?;
    assert_eq!(multi_block_read.unwrap(), multi_block);
    Ok(())
}

async fn step_4_append_with_flush(work_dir: &str) -> Result<()> {
    let mut hypercore = open_hypercore(work_dir).await?;
    for i in 0..5 {
        let append_outcome = hypercore.append(&[i]).await?;
        assert_eq!(append_outcome.length, (6 + i + 1) as u64);
        assert_eq!(append_outcome.byte_length, (12314 + i as u64 + 1) as u64);
    }
    Ok(())
}

async fn step_5_clear_some(work_dir: &str) -> Result<()> {
    let mut hypercore = open_hypercore(work_dir).await?;
    hypercore.clear(5, 6).await?;
    hypercore.clear(7, 9).await?;
    let info = hypercore.info();
    assert_eq!(info.length, 11);
    assert_eq!(info.byte_length, 12319);
    assert_eq!(info.contiguous_length, 5);
    let missing = hypercore.get(5).await?;
    assert_eq!(missing, None);
    let missing = hypercore.get(7).await?;
    assert_eq!(missing, None);
    let missing = hypercore.get(8).await?;
    assert_eq!(missing, None);
    let third = hypercore.get(4).await?;
    assert_eq!(third.unwrap(), b"third");
    Ok(())
}

async fn create_hypercore(work_dir: &str) -> Result<Hypercore<RandomAccessDisk>> {
    let path = Path::new(work_dir).to_owned();
    let key_pair = get_test_key_pair();
    let storage = Storage::new_disk(&path, true).await?;
    Ok(Builder::new(storage).set_key_pair(key_pair).build().await?)
}

async fn open_hypercore(work_dir: &str) -> Result<Hypercore<RandomAccessDisk>> {
    let path = Path::new(work_dir).to_owned();
    let storage = Storage::new_disk(&path, false).await?;
    Ok(Builder::new(storage).set_open(true).build().await?)
}

fn step_0_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: None,
        data: None,
        oplog: None,
        tree: None,
    }
}

fn step_1_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: None,
        data: None,
        oplog: Some("C5C042D47C25465FA708BB0384C88485E1C1AF848FC5D9E6DE34FAF1E88E41A9".into()),
        tree: None,
    }
}

fn step_2_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: Some("0E2E1FF956A39192CBB68D2212288FE75B32733AB0C442B9F0471E254A0382A2".into()),
        data: Some("872E4E50CE9990D8B041330C47C9DDD11BEC6B503AE9386A99DA8584E9BB12C4".into()),
        oplog: Some("E374F3CFEA62D333E3ADE22A24A0EA50E5AF09CF45E2DEDC0F56F5A214081156".into()),
        tree: Some("8577B24ADC763F65D562CD11204F938229AD47F27915B0821C46A0470B80813A".into()),
    }
}

fn step_3_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: Some("DEC1593A7456C8C9407B9B8B9C89682DFFF33C3892BCC9D9F06956FEE0A1B949".into()),
        data: Some("99EB5BC150A1102A7E50D15F90594660010B7FE719D54129065D1D417AA5015A".into()),
        oplog: Some("6CC9AB30A4146955886937FB442B86716C1E3C3517772EADA62ACF22D4922EAE".into()),
        tree: Some("38788609A8634DC8D34F9AE723F3169ADB20768ACFDFF266A43B7E217750DD1E".into()),
    }
}

fn step_4_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: Some("9B844E9378A7D13D6CDD4C1FF12FB313013E5CC472C6CB46497033563FE6B8F1".into()),
        data: Some("AF3AC31CFBE1733C62496CF8E856D5F1EFB4B06CBF1E74204221C89E2F3E1CDE".into()),
        oplog: Some("DD00EFAE87A5272281EB040ACCF387D0401895B32DEBE4E9313FCFD0CA2B76AE".into()),
        tree: Some("26339A21D606A1F731B90E8001030651D48378116B06A9C1EF87E2538194C2C6".into()),
    }
}

fn step_5_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: Some("40C9CED82AE0B7A397C9FDD14EEB7F70B74E8F1229F3ED931852591972DDC3E0".into()),
        data: Some("D9FFCCEEE9109751F034ECDAE328672956B90A6E0B409C3173741B8A5D0E75AB".into()),
        oplog: Some("AC0E5339F3DC58D7875A60B19D4B3AC7BA34990356CF7B8C1A0BC66FF8F31EFB".into()),
        tree: Some("26339A21D606A1F731B90E8001030651D48378116B06A9C1EF87E2538194C2C6".into()),
    }
}
