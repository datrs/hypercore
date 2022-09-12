mod common;
mod js;
use std::{path::Path, sync::Once};

use anyhow::Result;
use common::{create_hypercore_hash, get_test_key_pair};
#[cfg(feature = "v10")]
use hypercore::{Hypercore, Storage};
use js::{cleanup, install, js_run_step, prepare_test_set};

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

#[async_std::test]
#[cfg_attr(not(feature = "js_interop_tests"), ignore)]
#[cfg(feature = "v10")]
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
    Ok(())
}

#[async_std::test]
#[cfg_attr(not(feature = "js_interop_tests"), ignore)]
#[cfg(feature = "v10")]
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
    Ok(())
}

#[cfg(feature = "v10")]
async fn step_1_create(work_dir: &str) -> Result<()> {
    let path = Path::new(work_dir).to_owned();
    let key_pair = get_test_key_pair();
    let storage = Storage::new_disk(&path, false).await?;
    Hypercore::new_with_key_pair(storage, key_pair).await?;
    Ok(())
}

#[cfg(feature = "v10")]
async fn step_2_append_hello_world(work_dir: &str) -> Result<()> {
    let path = Path::new(work_dir).to_owned();
    let key_pair = get_test_key_pair();
    let storage = Storage::new_disk(&path, false).await?;
    let mut hypercore = Hypercore::new_with_key_pair(storage, key_pair).await?;
    let append_outcome = hypercore.append_batch(&[b"Hello", b"World"]).await?;
    assert_eq!(append_outcome.length, 2);
    assert_eq!(append_outcome.byte_length, 10);
    Ok(())
}

#[cfg(feature = "v10")]
async fn step_3_read_and_append_unflushed(work_dir: &str) -> Result<()> {
    let path = Path::new(work_dir).to_owned();
    let key_pair = get_test_key_pair();
    let storage = Storage::new_disk(&path, false).await?;
    let mut hypercore = Hypercore::new_with_key_pair(storage, key_pair).await?;
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
    let first = hypercore.get(2).await?;
    assert_eq!(first.unwrap(), b"first");
    let second = hypercore.get(3).await?;
    assert_eq!(second.unwrap(), b"second");
    let third = hypercore.get(3).await?;
    assert_eq!(third.unwrap(), b"third");
    Ok(())
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
        data: Some("402670849413BB97FAC3A322FC1EE3DE20F7F0D9C641B0F70BC4C619B3032C50".into()),
        oplog: Some("0536819D13798DADFCA9A8607D10DDD14254902FBCC35E97043039E4308869B5".into()),
        tree: Some("38788609A8634DC8D34F9AE723F3169ADB20768ACFDFF266A43B7E217750DD1E".into()),
    }
}
