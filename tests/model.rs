mod common;

#[cfg(feature = "v9")]
use common::create_feed;
use quickcheck::{quickcheck, Arbitrary, Gen};
use rand::seq::SliceRandom;
use rand::Rng;
use std::u8;

const MAX_FILE_SIZE: u64 = 5 * 10; // 5mb

#[derive(Clone, Debug)]
enum Op {
    Get {
        index: u64,
    },
    Append {
        data: Vec<u8>,
    },
    #[cfg(feature = "v9")]
    Verify,
    #[cfg(feature = "v10")]
    Clear {
        len_divisor_for_start: u8,
        len_divisor_for_length: u8,
    },
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
            #[cfg(feature = "v9")]
            2 => Op::Verify,
            #[cfg(feature = "v10")]
            2 => {
                let len_divisor_for_start: u8 = g.gen_range(1, 17);
                let len_divisor_for_length: u8 = g.gen_range(1, 17);
                Op::Clear {
                    len_divisor_for_start,
                    len_divisor_for_length,
                }
            }
            err => panic!("Invalid choice {}", err),
        }
    }
}

quickcheck! {

  #[cfg(feature = "v9")]
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
  #[cfg(all(feature = "v10", feature = "async-std"))]
  fn implementation_matches_model(ops: Vec<Op>) -> bool {
    async_std::task::block_on(assert_implementation_matches_model(ops))
  }

  #[cfg(all(feature = "v10", feature = "tokio"))]
  fn implementation_matches_model(ops: Vec<Op>) -> bool {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      assert_implementation_matches_model(ops).await
    })
  }
}

#[cfg(feature = "v10")]
async fn assert_implementation_matches_model(ops: Vec<Op>) -> bool {
    use hypercore::{Hypercore, Storage};

    let storage = Storage::new_memory()
        .await
        .expect("Memory storage creation should be successful");
    let mut hypercore = Hypercore::new(storage)
        .await
        .expect("Hypercore creation should be successful");

    let mut model: Vec<Option<Vec<u8>>> = vec![];

    for op in ops {
        match op {
            Op::Append { data } => {
                hypercore
                    .append(&data)
                    .await
                    .expect("Append should be successful");
                model.push(Some(data));
            }
            Op::Get { index } => {
                let data = hypercore
                    .get(index)
                    .await
                    .expect("Get should be successful");
                if index >= hypercore.info().length {
                    assert_eq!(data, None);
                } else {
                    assert_eq!(data, model[index as usize].clone());
                }
            }
            Op::Clear {
                len_divisor_for_start,
                len_divisor_for_length,
            } => {
                let start = {
                    let result = model.len() as u64 / len_divisor_for_start as u64;
                    if result == model.len() as u64 {
                        if model.len() > 0 {
                            result - 1
                        } else {
                            0
                        }
                    } else {
                        result
                    }
                };
                let length = model.len() as u64 / len_divisor_for_length as u64;
                let end = start + length;
                let model_end = if end < model.len() as u64 {
                    end
                } else {
                    model.len() as u64
                };
                hypercore
                    .clear(start, end)
                    .await
                    .expect("Clear should be successful");
                model[start as usize..model_end as usize].fill(None);
            }
        }
    }
    true
}
