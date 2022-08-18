mod common;
mod js;
use std::{path::Path, sync::Once};

use ed25519_dalek::{PublicKey, SecretKey, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use hypercore::{FeedBuilder, Storage};
use js::{cleanup, install, js_step_1_create_hypercore, prepare_test_set};

const TEST_SET_JS_FIRST: &str = "jsfirst";
const TEST_SET_RS_FIRST: &str = "rsfirst";

const TEST_PUBLIC_KEY_BYTES: [u8; PUBLIC_KEY_LENGTH] = [
    0x97, 0x60, 0x6c, 0xaa, 0xd2, 0xb0, 0x8c, 0x1d, 0x5f, 0xe1, 0x64, 0x2e, 0xee, 0xa5, 0x62, 0xcb,
    0x91, 0xd6, 0x55, 0xe2, 0x00, 0xc8, 0xd4, 0x3a, 0x32, 0x09, 0x1d, 0x06, 0x4a, 0x33, 0x1e, 0xe3,
];
// NB: In the javascript version this is 64 bytes, but that's because sodium appends the the public
// key after the secret key for some reason. Only the first 32 bytes are actually used in
// javascript side too for signing.
const TEST_SECRET_KEY_BYTES: [u8; SECRET_KEY_LENGTH] = [
    0x27, 0xe6, 0x74, 0x25, 0xc1, 0xff, 0xd1, 0xd9, 0xee, 0x62, 0x5c, 0x96, 0x2b, 0x57, 0x13, 0xc3,
    0x51, 0x0b, 0x71, 0x14, 0x15, 0xf3, 0x31, 0xf6, 0xfa, 0x9e, 0xf2, 0xbf, 0x23, 0x5f, 0x2f, 0xfe,
];

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
    let hash = common::create_hypercore_hash(work_dir).expect("Could not hash directory");
    assert_eq!(get_step_1_hash(), hash)
}

#[async_std::test]
#[cfg_attr(not(feature = "js_interop_tests"), ignore)]
async fn js_interop_rs_first() {
    init();
    let work_dir = prepare_test_set(TEST_SET_RS_FIRST);
    step_1_create_hypercore(&work_dir).await;
    let _hash = common::create_hypercore_hash(work_dir).expect("Could not hash directory");
    // TODO: Make this match, only data does right now
    // assert_eq!(get_step_1_hash(), hash)
}

async fn step_1_create_hypercore(work_dir: &str) {
    let path = Path::new(work_dir).to_owned();
    let storage = Storage::new_disk(&path, false).await.unwrap();

    let public_key = PublicKey::from_bytes(&TEST_PUBLIC_KEY_BYTES).unwrap();
    let secret_key = SecretKey::from_bytes(&TEST_SECRET_KEY_BYTES).unwrap();

    let builder = FeedBuilder::new(public_key, storage);
    let mut feed = builder.secret_key(secret_key).build().await.unwrap();

    feed.append(b"Hello").await.unwrap();
    feed.append(b"World").await.unwrap();
    drop(feed);
}

fn get_step_1_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: "0E2E1FF956A39192CBB68D2212288FE75B32733AB0C442B9F0471E254A0382A2".into(),
        data: "872E4E50CE9990D8B041330C47C9DDD11BEC6B503AE9386A99DA8584E9BB12C4".into(),
        oplog: "E374F3CFEA62D333E3ADE22A24A0EA50E5AF09CF45E2DEDC0F56F5A214081156".into(),
        tree: "8577B24ADC763F65D562CD11204F938229AD47F27915B0821C46A0470B80813A".into(),
    }
}
