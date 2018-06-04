#[macro_use]
extern crate quickcheck;
extern crate hypercore;

mod helpers;

use helpers::create_feed;
use quickcheck::{Arbitrary, Gen};
use std::u8;

const MAX_FILE_SIZE: usize = 5 * 10; // 5mb

#[derive(Clone, Debug)]
enum Op {
  Get { index: usize },
  Append { data: Vec<u8> },
}

impl Arbitrary for Op {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {

    if g.gen::<bool>() {
      let index: usize = g.gen_range(0, MAX_FILE_SIZE);
      Op::Get { index }
    } else {
      let length: usize = g.gen_range(0, MAX_FILE_SIZE / 3);
      let mut data = Vec::with_capacity(length);
      for _ in 0..length {
        data.push(u8::arbitrary(g));
      }
      Op::Append { data }
    }
  }
}

quickcheck! {
  fn implementation_matches_model(ops: Vec<Op>) -> bool {
    let page_size = 50;

    let mut insta = create_feed(page_size)
      .expect("Instance creation should be successful");
    let mut model = vec![];

    for op in ops {
      match op {
        Op::Append { data } => {
          insta.append(&data).expect("Append should be successful");
          model.push(data);
        },
        Op::Get { index } => {
          let data = insta.get(index).expect("Get should be successful");
          if index >= insta.len() {
            assert_eq!(data, None);
          } else {
            assert_eq!(data, Some(model[index].clone()));
          }
        },
      }
    }
    true
  }
}
