mod common;
mod js;
use std::{path::Path, sync::Once};

use anyhow::Result;
use common::{create_hypercore_hash, get_test_key_pair};
#[cfg(feature = "v10")]
use hypercore::{Hypercore, Storage};
use js::{cleanup, install, js_step_1_create, js_step_2_append_hello_world, prepare_test_set};

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

#[test]
#[cfg_attr(not(feature = "js_interop_tests"), ignore)]
fn js_interop_js_first() {
    init();
    let work_dir = prepare_test_set(TEST_SET_JS_FIRST);
    assert_eq!(get_step_0_hash(), create_hypercore_hash(&work_dir));
    js_step_1_create(TEST_SET_JS_FIRST);
    assert_eq!(get_step_1_hash(), create_hypercore_hash(&work_dir))
}

#[async_std::test]
#[cfg_attr(not(feature = "js_interop_tests"), ignore)]
#[cfg(feature = "v10")]
async fn js_interop_rs_first() -> Result<()> {
    init();
    let work_dir = prepare_test_set(TEST_SET_RS_FIRST);
    assert_eq!(get_step_0_hash(), create_hypercore_hash(&work_dir));
    step_1_create(&work_dir).await?;
    assert_eq!(get_step_1_hash(), create_hypercore_hash(&work_dir));
    js_step_2_append_hello_world(TEST_SET_RS_FIRST);
    assert_eq!(get_step_2_hash(), create_hypercore_hash(&work_dir));
    Ok(())
}

#[cfg(feature = "v10")]
async fn step_1_create(work_dir: &str) -> Result<()> {
    let path = Path::new(work_dir).to_owned();
    let key_pair = get_test_key_pair();
    let storage = Storage::new_disk(&path, false).await?;
    let _hypercore = Hypercore::new_with_key_pair(storage, key_pair).await?;

    // let builder = FeedBuilder::new(public_key, storage);
    // let mut feed = builder.secret_key(secret_key).build().await.unwrap();

    // feed.append(b"Hello").await.unwrap();
    // feed.append(b"World").await.unwrap();
    // drop(feed);
    Ok(())
}

fn get_step_0_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: None,
        data: None,
        oplog: None,
        tree: None,
    }
}

fn get_step_1_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: None,
        data: None,
        oplog: Some("C5C042D47C25465FA708BB0384C88485E1C1AF848FC5D9E6DE34FAF1E88E41A9".into()),
        tree: None,
    }
}

fn get_step_2_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: Some("0E2E1FF956A39192CBB68D2212288FE75B32733AB0C442B9F0471E254A0382A2".into()),
        data: Some("872E4E50CE9990D8B041330C47C9DDD11BEC6B503AE9386A99DA8584E9BB12C4".into()),
        oplog: Some("E374F3CFEA62D333E3ADE22A24A0EA50E5AF09CF45E2DEDC0F56F5A214081156".into()),
        tree: Some("8577B24ADC763F65D562CD11204F938229AD47F27915B0821C46A0470B80813A".into()),
    }
}
