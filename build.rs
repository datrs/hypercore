extern crate cbindgen;

use cbindgen::{Builder, Config};
use std::env;

fn main() {
  let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
  let config = Config::from_file("cbindgen.toml")
    .expect("Failed to read `cbindgen.toml`!");

  cbindgen::Builder::new()
    .with_crate(crate_dir)
    .with_config(config)
    .generate()
    .expect("Unable to generate bindings")
    .write_to_file("include/hypercore.h");
}
