mod common;

use common::create_feed;
use quickcheck::{quickcheck, Arbitrary, Gen};
use rand::seq::SliceRandom;
use rand::Rng;
use std::u8;

const MAX_FILE_SIZE: u64 = 5 * 10; // 5mb

#[derive(Clone, Debug)]
enum Op {
    Get { index: u64 },
    Append { data: Vec<u8> },
    Verify,
}

impl Arbitrary for Op {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let choices = [0, 1, 2];
        match choices.choose(g).expect("Value should exist") {
            0 => {
                let index: u64 = g.gen_range(0, MAX_FILE_SIZE);
                Op::Get { index }
            }
            1 => {
                let length: u64 = g.gen_range(0, MAX_FILE_SIZE / 3);
                let mut data = Vec::with_capacity(length as usize);
                for _ in 0..length {
                    data.push(u8::arbitrary(g));
                }
                Op::Append { data }
            }
            2 => Op::Verify,
            err => panic!("Invalid choice {}", err),
        }
    }
}

quickcheck! {
  fn implementation_matches_model(ops: Vec<Op>) -> bool {
    async_std::task::block_on(async {
      let page_size = 50;

      let mut insta = create_feed(page_size)
        .await
        .expect("Instance creation should be successful");
      let mut model = vec![];

      for op in ops {
        match op {
          Op::Append { data } => {
            insta.append(&data).await.expect("Append should be successful");
            model.push(data);
          },
          Op::Get { index } => {
            let data = insta.get(index).await.expect("Get should be successful");
            if index >= insta.len() {
              assert_eq!(data, None);
            } else {
              assert_eq!(data, Some(model[index as usize].clone()));
            }
          },
          Op::Verify => {
            let len = insta.len();
            if len == 0 {
              insta.signature(len).await.unwrap_err();
            } else {
              // Always test index of last entry, which is `len - 1`.
              let len = len - 1;
              let sig = insta.signature(len).await.expect("Signature should exist");
              insta.verify(len, &sig).await.expect("Signature should match");
            }
          },
        }
      }
      true
    })
  }
}
