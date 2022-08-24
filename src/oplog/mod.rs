use crate::compact_encoding::{CompactEncoding, State};
use crate::crypto::{generate_keypair, PublicKey, SecretKey};
use crate::PartialKeypair;

/// Oplog header
#[derive(Debug)]
struct Header {
    types: HeaderTypes,
    tree: HeaderTree,
    signer: PartialKeypair,
    hints: HeaderHints,
    contiguous_length: u64,
}

impl Header {
    /// Creates a new Header from given key pair
    pub fn new(key_pair: PartialKeypair) -> Header {
        Header {
            types: HeaderTypes {
                tree: "blake2b".to_string(),
                bitfield: "raw".to_string(),
                signer: "ed25519".to_string(),
            },
            tree: HeaderTree { fork: 0, length: 0 },
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
#[derive(Debug)]
struct HeaderTypes {
    tree: String,
    bitfield: String,
    signer: String,
}

impl CompactEncoding<HeaderTypes> for State {
    fn preencode(&mut self, value: &HeaderTypes) {
        self.preencode(&value.tree);
        self.preencode(&value.bitfield);
        self.preencode(&value.signer);
    }

    fn encode(&mut self, value: &HeaderTypes, buffer: &mut Box<[u8]>) {
        self.encode(&value.tree, buffer);
        self.encode(&value.bitfield, buffer);
        self.encode(&value.signer, buffer);
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> HeaderTypes {
        let tree = self.decode(buffer);
        let bitfield = self.decode(buffer);
        let signer = self.decode(buffer);
        HeaderTypes {
            tree,
            bitfield,
            signer,
        }
    }
}

/// Oplog header tree
#[derive(Debug)]
struct HeaderTree {
    fork: u64,
    length: u64,
}

impl CompactEncoding<HeaderTree> for State {
    fn preencode(&mut self, value: &HeaderTree) {
        self.preencode(&value.fork);
        self.preencode(&value.length);
    }

    fn encode(&mut self, value: &HeaderTree, buffer: &mut Box<[u8]>) {
        self.encode(&value.fork, buffer);
        self.encode(&value.length, buffer);
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> HeaderTree {
        let fork = self.decode(buffer);
        let length = self.decode(buffer);
        HeaderTree { fork, length }
    }
}

impl CompactEncoding<PartialKeypair> for State {
    fn preencode(&mut self, value: &PartialKeypair) {
        let public_key_bytes: Box<[u8]> = value.public.as_bytes().to_vec().into_boxed_slice();
        self.preencode(&public_key_bytes);
        match &value.secret {
            Some(secret_key) => {
                let secret_key_bytes: Box<[u8]> = secret_key.as_bytes().to_vec().into_boxed_slice();
                self.preencode(&secret_key_bytes);
            }
            None => {
                self.end += 1;
            }
        }
    }

    fn encode(&mut self, value: &PartialKeypair, buffer: &mut Box<[u8]>) {
        let public_key_bytes: Box<[u8]> = value.public.as_bytes().to_vec().into_boxed_slice();
        self.encode(&public_key_bytes, buffer);
        match &value.secret {
            Some(secret_key) => {
                let secret_key_bytes: Box<[u8]> = secret_key.as_bytes().to_vec().into_boxed_slice();
                self.encode(&secret_key_bytes, buffer);
            }
            None => {
                buffer[self.start] = 0;
                self.start += 1;
            }
        }
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> PartialKeypair {
        let public_key_bytes: Box<[u8]> = self.decode(buffer);
        let secret_key_bytes: Box<[u8]> = self.decode(buffer);
        let secret: Option<SecretKey> = if secret_key_bytes.len() == 0 {
            None
        } else {
            Some(SecretKey::from_bytes(&secret_key_bytes).unwrap())
        };

        PartialKeypair {
            public: PublicKey::from_bytes(&public_key_bytes).unwrap(),
            secret,
        }
    }
}

/// Oplog header hints
#[derive(Debug)]
struct HeaderHints {
    reorgs: Vec<String>,
}

impl CompactEncoding<HeaderHints> for State {
    fn preencode(&mut self, value: &HeaderHints) {
        self.preencode(&value.reorgs);
    }

    fn encode(&mut self, value: &HeaderHints, buffer: &mut Box<[u8]>) {
        self.encode(&value.reorgs, buffer);
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> HeaderHints {
        HeaderHints {
            reorgs: self.decode(buffer),
        }
    }
}

impl CompactEncoding<Header> for State {
    fn preencode(&mut self, value: &Header) {
        self.start += 1; // Version
        self.preencode(&value.types);
        // TODO self.preencode(&value.user_data);
        self.preencode(&value.tree);
        self.preencode(&value.signer);
        self.preencode(&value.hints);
        self.preencode(&value.contiguous_length);
    }

    fn encode(&mut self, value: &Header, buffer: &mut Box<[u8]>) {
        buffer[0] = 0; // Version
        self.start += 1;
        self.encode(&value.types, buffer);
        // TODO self.encode(&value.user_data, buffer);
        self.encode(&value.tree, buffer);
        self.encode(&value.signer, buffer);
        self.encode(&value.hints, buffer);
        self.encode(&value.contiguous_length, buffer);
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> Header {
        let version: usize = self.decode(buffer);
        if version != 0 {
            panic!("Unknown oplog version {}", version);
        }
        let types: HeaderTypes = self.decode(buffer);
        // TODO: let user_data: HeaderUserData = self.decode(buffer);
        let tree: HeaderTree = self.decode(buffer);
        let signer: PartialKeypair = self.decode(buffer);
        let hints: HeaderHints = self.decode(buffer);
        let contiguous_length: u64 = self.decode(buffer);

        Header {
            types,
            tree,
            signer,
            hints,
            contiguous_length,
        }
    }
}

/// Oplog
#[derive(Debug)]
pub struct Oplog {
    #[allow(dead_code)]
    header: Header,
}

impl Oplog {
    /// Opens an new Oplog from given key pair and existing content as a byte buffer
    #[allow(dead_code)]
    pub fn open(key_pair: PartialKeypair, existing: Box<[u8]>) -> Oplog {
        if existing.len() == 0 {
            Oplog {
                header: Header::new(key_pair),
            }
        } else {
            unimplemented!("Reading an exising oplog is not supported yet");
        }
    }
}
