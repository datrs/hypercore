#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]
#![feature(external_doc)]
#![doc(include = "../README.md")]
// #![cfg_attr(test, feature(plugin))]
// #![cfg_attr(test, plugin(clippy))]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate failure;

pub mod bitfield;
pub mod crypto;
pub mod storage;

mod feed;

pub use feed::*;
