// lazy_static! {
//   static ref LEAF_TYPE: &'static [u8] = b"0";
//   static ref PARENT_TYPE: &'static [u8] = b"1";
//   static ref ROOT_TYPE: &'static [u8] = b"2";
//   static ref HYPERCORE: &'static [u8] = b"hypercore";
// }

/// Generate an Ed25519 keypair.
pub mod key_pair;

/// Create an Ed25519 signature for data.
pub mod signature;
