mod common;
mod js;
use js::{cleanup, init, step_1_create_hypercore};

const WORK_DIR: &str = "tests/js/work";
const TEST_SET_BASIC: &str = "basic";

#[test]
#[cfg_attr(not(feature = "js_interop_tests"), ignore)]
fn basic_interop_with_javascript() {
    cleanup();
    init(TEST_SET_BASIC);
    step_1_create_hypercore(TEST_SET_BASIC);
    let hash = common::create_hypercore_hash(format!("{}/{}", WORK_DIR, TEST_SET_BASIC))
        .expect("Could not hash directory");
    assert_eq!(get_step_1_hash(), hash)
}

fn get_step_1_hash() -> common::HypercoreHash {
    common::HypercoreHash {
        bitfield: "0E2E1FF956A39192CBB68D2212288FE75B32733AB0C442B9F0471E254A0382A2".into(),
        data: "872E4E50CE9990D8B041330C47C9DDD11BEC6B503AE9386A99DA8584E9BB12C4".into(),
        oplog: "E374F3CFEA62D333E3ADE22A24A0EA50E5AF09CF45E2DEDC0F56F5A214081156".into(),
        tree: "8577B24ADC763F65D562CD11204F938229AD47F27915B0821C46A0470B80813A".into(),
    }
}
