mod common;
mod js;
use std::{path::Path, sync::Once};

use common::{create_hypercore_hash, get_test_key_pair};
use js::{cleanup, install, js_step_1_create_hypercore, prepare_test_set};

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
    js_step_1_create_hypercore(TEST_SET_JS_FIRST);
    let hash = create_hypercore_hash(work_dir).expect("Could not hash directory");
    assert_eq!(get_step_1_hash(), hash)
}

#[async_std::test]
#[cfg_attr(not(feature = "js_interop_tests"), ignore)]
async fn js_interop_rs_first() {
    init();
    let work_dir = prepare_test_set(TEST_SET_RS_FIRST);
    step_1_create_hypercore(&work_dir).await;
    let _hash = create_hypercore_hash(work_dir).expect("Could not hash directory");
    // TODO: Make this match, only data does right now
    // assert_eq!(get_step_1_hash(), hash)
}

async fn step_1_create_hypercore(work_dir: &str) {
    let path = Path::new(work_dir).to_owned();
    let storage = get_test_key_pair();

    // let builder = FeedBuilder::new(public_key, storage);
    // let mut feed = builder.secret_key(secret_key).build().await.unwrap();

    // feed.append(b"Hello").await.unwrap();
    // feed.append(b"World").await.unwrap();
    // drop(feed);
}

fn get_step_1_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: "0E2E1FF956A39192CBB68D2212288FE75B32733AB0C442B9F0471E254A0382A2".into(),
        data: "872E4E50CE9990D8B041330C47C9DDD11BEC6B503AE9386A99DA8584E9BB12C4".into(),
        oplog: "E374F3CFEA62D333E3ADE22A24A0EA50E5AF09CF45E2DEDC0F56F5A214081156".into(),
        tree: "8577B24ADC763F65D562CD11204F938229AD47F27915B0821C46A0470B80813A".into(),
    }
}
