use compact_encoding::{CompactEncoding, State};

use crate::crypto::{PublicKey, SecretKey};
use crate::PartialKeypair;

/// Oplog header.
#[derive(Debug, Clone)]
pub struct Header {
    pub(crate) types: HeaderTypes,
    // TODO: This is a keyValueArray in JS
    pub(crate) user_data: Vec<String>,
    pub(crate) tree: HeaderTree,
    pub(crate) signer: PartialKeypair,
    pub(crate) hints: HeaderHints,
    pub(crate) contiguous_length: u64,
}

impl Header {
    /// Creates a new Header from given key pair
    pub fn new(key_pair: PartialKeypair) -> Self {
        Self {
            types: HeaderTypes::new(),
            user_data: vec![],
            tree: HeaderTree::new(),
            signer: key_pair,
            hints: HeaderHints { reorgs: vec![] },
            contiguous_length: 0,
        }
        // Javascript side, initial header
        //
        // header = {
        //   types: { tree: 'blake2b', bitfield: 'raw', signer: 'ed25519' },
        //   userData: [],
        //   tree: {
        //     fork: 0,
        //     length: 0,
        //     rootHash: null,
        //     signature: null
        //   },
        //   signer: opts.keyPair || crypto.keyPair(),
        //   hints: {
        //     reorgs: []
        //   },
        //   contiguousLength: 0
        // }
    }
}

/// Oplog header types
#[derive(Debug, PartialEq, Clone)]
pub struct HeaderTypes {
    pub(crate) tree: String,
    pub(crate) bitfield: String,
    pub(crate) signer: String,
}
impl HeaderTypes {
    pub fn new() -> Self {
        Self {
            tree: "blake2b".to_string(),
            bitfield: "raw".to_string(),
            signer: "ed25519".to_string(),
        }
    }
}

impl CompactEncoding<HeaderTypes> for State {
    fn preencode(&mut self, value: &HeaderTypes) {
        self.preencode(&value.tree);
        self.preencode(&value.bitfield);
        self.preencode(&value.signer);
    }

    fn encode(&mut self, value: &HeaderTypes, buffer: &mut [u8]) {
        self.encode(&value.tree, buffer);
        self.encode(&value.bitfield, buffer);
        self.encode(&value.signer, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> HeaderTypes {
        let tree: String = self.decode(buffer);
        let bitfield: String = self.decode(buffer);
        let signer: String = self.decode(buffer);
        HeaderTypes {
            tree,
            bitfield,
            signer,
        }
    }
}

/// Oplog header tree
#[derive(Debug, PartialEq, Clone)]
pub struct HeaderTree {
    pub(crate) fork: u64,
    pub(crate) length: u64,
    pub(crate) root_hash: Box<[u8]>,
    pub(crate) signature: Box<[u8]>,
}

impl HeaderTree {
    pub fn new() -> Self {
        Self {
            fork: 0,
            length: 0,
            root_hash: Box::new([]),
            signature: Box::new([]),
        }
    }
}

impl CompactEncoding<HeaderTree> for State {
    fn preencode(&mut self, value: &HeaderTree) {
        self.preencode(&value.fork);
        self.preencode(&value.length);
        self.preencode(&value.root_hash);
        self.preencode(&value.signature);
    }

    fn encode(&mut self, value: &HeaderTree, buffer: &mut [u8]) {
        self.encode(&value.fork, buffer);
        self.encode(&value.length, buffer);
        self.encode(&value.root_hash, buffer);
        self.encode(&value.signature, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> HeaderTree {
        let fork: u64 = self.decode(buffer);
        let length: u64 = self.decode(buffer);
        let root_hash: Box<[u8]> = self.decode(buffer);
        let signature: Box<[u8]> = self.decode(buffer);
        HeaderTree {
            fork,
            length,
            root_hash,
            signature,
        }
    }
}

/// NB: In Javascript's sodium the secret key contains in itself also the public key, so to
/// maintain binary compatibility, we store the public key in the oplog now twice.
impl CompactEncoding<PartialKeypair> for State {
    fn preencode(&mut self, value: &PartialKeypair) {
        self.end += 1 + 32;
        match &value.secret {
            Some(_) => {
                // Also add room for the public key
                self.end += 1 + 64;
            }
            None => {
                self.end += 1;
            }
        }
    }

    fn encode(&mut self, value: &PartialKeypair, buffer: &mut [u8]) {
        let public_key_bytes: Box<[u8]> = value.public.as_bytes().to_vec().into_boxed_slice();
        self.encode(&public_key_bytes, buffer);
        match &value.secret {
            Some(secret_key) => {
                let mut secret_key_bytes: Vec<u8> = Vec::with_capacity(64);
                secret_key_bytes.extend_from_slice(secret_key.as_bytes());
                secret_key_bytes.extend_from_slice(&public_key_bytes);
                let secret_key_bytes: Box<[u8]> = secret_key_bytes.into_boxed_slice();
                self.encode(&secret_key_bytes, buffer);
            }
            None => {
                buffer[self.start] = 0;
                self.start += 1;
            }
        }
    }

    fn decode(&mut self, buffer: &[u8]) -> PartialKeypair {
        let public_key_bytes: Box<[u8]> = self.decode(buffer);
        let secret_key_bytes: Box<[u8]> = self.decode(buffer);
        let secret: Option<SecretKey> = if secret_key_bytes.len() == 0 {
            None
        } else {
            Some(SecretKey::from_bytes(&secret_key_bytes[0..32]).unwrap())
        };

        PartialKeypair {
            public: PublicKey::from_bytes(&public_key_bytes).unwrap(),
            secret,
        }
    }
}

/// Oplog header hints
#[derive(Debug, Clone)]
pub struct HeaderHints {
    pub(crate) reorgs: Vec<String>,
}

impl CompactEncoding<HeaderHints> for State {
    fn preencode(&mut self, value: &HeaderHints) {
        self.preencode(&value.reorgs);
    }

    fn encode(&mut self, value: &HeaderHints, buffer: &mut [u8]) {
        self.encode(&value.reorgs, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> HeaderHints {
        HeaderHints {
            reorgs: self.decode(buffer),
        }
    }
}

impl CompactEncoding<Header> for State {
    fn preencode(&mut self, value: &Header) {
        self.end += 1; // Version
        self.preencode(&value.types);
        self.preencode(&value.user_data);
        self.preencode(&value.tree);
        self.preencode(&value.signer);
        self.preencode(&value.hints);
        self.preencode(&value.contiguous_length);
    }

    fn encode(&mut self, value: &Header, buffer: &mut [u8]) {
        buffer[self.start] = 0; // Version
        self.start += 1;
        self.encode(&value.types, buffer);
        self.encode(&value.user_data, buffer);
        self.encode(&value.tree, buffer);
        self.encode(&value.signer, buffer);
        self.encode(&value.hints, buffer);
        self.encode(&value.contiguous_length, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> Header {
        let version: u8 = buffer[self.start];
        self.start += 1;
        if version != 0 {
            panic!("Unknown oplog version {}", version);
        }
        let types: HeaderTypes = self.decode(buffer);
        let user_data: Vec<String> = self.decode(buffer);
        let tree: HeaderTree = self.decode(buffer);
        let signer: PartialKeypair = self.decode(buffer);
        let hints: HeaderHints = self.decode(buffer);
        let contiguous_length: u64 = self.decode(buffer);

        Header {
            types,
            user_data,
            tree,
            signer,
            hints,
            contiguous_length,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::crypto::generate_keypair;

    #[test]
    fn encode_header_types() {
        let mut enc_state = State::new_with_start_and_end(8, 8);
        let header_types = HeaderTypes::new();
        enc_state.preencode(&header_types);
        let mut buffer = enc_state.create_buffer();
        enc_state.encode(&header_types, &mut buffer);
        let mut dec_state = State::from_buffer(&buffer);
        dec_state.start = 8;
        let header_types_ret: HeaderTypes = dec_state.decode(&buffer);
        assert_eq!(header_types, header_types_ret);
    }

    #[test]
    fn encode_partial_key_pair() {
        let mut enc_state = State::new();
        let key_pair = generate_keypair();
        let key_pair = PartialKeypair {
            public: key_pair.public,
            secret: Some(key_pair.secret),
        };
        enc_state.preencode(&key_pair);
        let mut buffer = enc_state.create_buffer();
        // Pub key: 1 byte for length, 32 bytes for content
        // Sec key: 1 byte for length, 64 bytes for data
        let expected_len = 1 + 32 + 1 + 64;
        assert_eq!(buffer.len(), expected_len);
        assert_eq!(enc_state.end, expected_len);
        assert_eq!(enc_state.start, 0);
        enc_state.encode(&key_pair, &mut buffer);
        let mut dec_state = State::from_buffer(&buffer);
        let key_pair_ret: PartialKeypair = dec_state.decode(&buffer);
        assert_eq!(key_pair.public, key_pair_ret.public);
        assert_eq!(
            key_pair.secret.unwrap().as_bytes(),
            key_pair_ret.secret.unwrap().as_bytes()
        );
    }

    #[test]
    fn encode_tree() {
        let mut enc_state = State::new();
        let tree = HeaderTree::new();
        enc_state.preencode(&tree);
        let mut buffer = enc_state.create_buffer();
        enc_state.encode(&tree, &mut buffer);
        let mut dec_state = State::from_buffer(&buffer);
        let tree_ret: HeaderTree = dec_state.decode(&buffer);
        assert_eq!(tree, tree_ret);
    }

    #[test]
    fn encode_header() {
        let mut enc_state = State::new();
        let key_pair = generate_keypair();
        let key_pair = PartialKeypair {
            public: key_pair.public,
            secret: Some(key_pair.secret),
        };
        let header = Header::new(key_pair);
        enc_state.preencode(&header);
        let mut buffer = enc_state.create_buffer();
        enc_state.encode(&header, &mut buffer);
        let mut dec_state = State::from_buffer(&buffer);
        let header_ret: Header = dec_state.decode(&buffer);
        assert_eq!(header.signer.public, header_ret.signer.public);
        assert_eq!(header.tree.fork, header_ret.tree.fork);
        assert_eq!(header.tree.length, header_ret.tree.length);
        assert_eq!(header.types, header_ret.types);
    }
}
