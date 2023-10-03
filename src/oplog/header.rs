use compact_encoding::EncodingErrorKind;
use compact_encoding::{CompactEncoding, EncodingError, State};
use ed25519_dalek::{SigningKey, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use std::convert::TryInto;

use crate::crypto::default_signer_manifest;
use crate::crypto::Manifest;
use crate::PartialKeypair;
use crate::VerifyingKey;

/// Oplog header.
#[derive(Debug, Clone)]
pub(crate) struct Header {
    // TODO: v11 has external
    // pub(crate) external: Option<bool>,
    // NB: This is the manifest hash in v11, right now
    // just the public key,
    pub(crate) key: [u8; 32],
    pub(crate) manifest: Manifest,
    pub(crate) key_pair: PartialKeypair,
    // TODO: This is a keyValueArray in JS
    pub(crate) user_data: Vec<String>,
    pub(crate) tree: HeaderTree,
    pub(crate) hints: HeaderHints,
}

impl Header {
    /// Creates a new Header from given key pair
    pub(crate) fn new(key_pair: PartialKeypair) -> Self {
        let key = key_pair.public.to_bytes();
        let manifest = default_signer_manifest(key);
        Self {
            key,
            manifest,
            key_pair,
            user_data: vec![],
            tree: HeaderTree::new(),
            hints: HeaderHints {
                reorgs: vec![],
                contiguous_length: 0,
            },
        }
        // Javascript side, initial header
        // header = {
        //    external: null,
        //    key: opts.key || (compat ? manifest.signer.publicKey : manifestHash(manifest)),
        //    manifest,
        //    keyPair,
        //    userData: [],
        //    tree: {
        //      fork: 0,
        //      length: 0,
        //      rootHash: null,
        //      signature: null
        //    },
        //    hints: {
        //      reorgs: [],
        //      contiguousLength: 0
        //    }
        //  }
    }
}

/// Oplog header tree
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct HeaderTree {
    pub(crate) fork: u64,
    pub(crate) length: u64,
    pub(crate) root_hash: Box<[u8]>,
    pub(crate) signature: Box<[u8]>,
}

impl HeaderTree {
    pub(crate) fn new() -> Self {
        Self {
            fork: 0,
            length: 0,
            root_hash: Box::new([]),
            signature: Box::new([]),
        }
    }
}

impl CompactEncoding<HeaderTree> for State {
    fn preencode(&mut self, value: &HeaderTree) -> Result<usize, EncodingError> {
        self.preencode(&value.fork)?;
        self.preencode(&value.length)?;
        self.preencode(&value.root_hash)?;
        self.preencode(&value.signature)
    }

    fn encode(&mut self, value: &HeaderTree, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.encode(&value.fork, buffer)?;
        self.encode(&value.length, buffer)?;
        self.encode(&value.root_hash, buffer)?;
        self.encode(&value.signature, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<HeaderTree, EncodingError> {
        let fork: u64 = self.decode(buffer)?;
        let length: u64 = self.decode(buffer)?;
        let root_hash: Box<[u8]> = self.decode(buffer)?;
        let signature: Box<[u8]> = self.decode(buffer)?;
        Ok(HeaderTree {
            fork,
            length,
            root_hash,
            signature,
        })
    }
}

/// NB: In Javascript's sodium the secret key contains in itself also the public key, so to
/// maintain binary compatibility, we store the public key in the oplog now twice.
impl CompactEncoding<PartialKeypair> for State {
    fn preencode(&mut self, value: &PartialKeypair) -> Result<usize, EncodingError> {
        self.add_end(1 + PUBLIC_KEY_LENGTH)?;
        match &value.secret {
            Some(_) => {
                // Also add room for the public key
                self.add_end(1 + SECRET_KEY_LENGTH + PUBLIC_KEY_LENGTH)
            }
            None => self.add_end(1),
        }
    }

    fn encode(
        &mut self,
        value: &PartialKeypair,
        buffer: &mut [u8],
    ) -> Result<usize, EncodingError> {
        let public_key_bytes: Box<[u8]> = value.public.as_bytes().to_vec().into_boxed_slice();
        self.encode(&public_key_bytes, buffer)?;
        match &value.secret {
            Some(secret_key) => {
                let mut secret_key_bytes: Vec<u8> =
                    Vec::with_capacity(SECRET_KEY_LENGTH + PUBLIC_KEY_LENGTH);
                secret_key_bytes.extend_from_slice(&secret_key.to_bytes());
                secret_key_bytes.extend_from_slice(&public_key_bytes);
                let secret_key_bytes: Box<[u8]> = secret_key_bytes.into_boxed_slice();
                self.encode(&secret_key_bytes, buffer)
            }
            None => self.set_byte_to_buffer(0, buffer),
        }
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<PartialKeypair, EncodingError> {
        let public_key_bytes: Box<[u8]> = self.decode(buffer)?;
        let public_key_bytes: [u8; PUBLIC_KEY_LENGTH] =
            public_key_bytes[0..PUBLIC_KEY_LENGTH].try_into().unwrap();
        let secret_key_bytes: Box<[u8]> = self.decode(buffer)?;
        let secret: Option<SigningKey> = if secret_key_bytes.is_empty() {
            None
        } else {
            let secret_key_bytes: [u8; SECRET_KEY_LENGTH] =
                secret_key_bytes[0..SECRET_KEY_LENGTH].try_into().unwrap();
            Some(SigningKey::from_bytes(&secret_key_bytes))
        };

        Ok(PartialKeypair {
            public: VerifyingKey::from_bytes(&public_key_bytes).unwrap(),
            secret,
        })
    }
}

/// Oplog header hints
#[derive(Debug, Clone)]
pub(crate) struct HeaderHints {
    pub(crate) reorgs: Vec<String>,
    pub(crate) contiguous_length: u64,
}

impl CompactEncoding<HeaderHints> for State {
    fn preencode(&mut self, value: &HeaderHints) -> Result<usize, EncodingError> {
        self.preencode(&value.reorgs)?;
        self.preencode(&value.contiguous_length)
    }

    fn encode(&mut self, value: &HeaderHints, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.encode(&value.reorgs, buffer)?;
        self.encode(&value.contiguous_length, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<HeaderHints, EncodingError> {
        Ok(HeaderHints {
            reorgs: self.decode(buffer)?,
            contiguous_length: self.decode(buffer)?,
        })
    }
}

impl CompactEncoding<Header> for State {
    fn preencode(&mut self, value: &Header) -> Result<usize, EncodingError> {
        self.add_end(1)?; // Version
        self.add_end(1)?; // Flags
        self.preencode_fixed_32()?; // key
        self.preencode(&value.manifest)?;
        self.preencode(&value.key_pair)?;
        self.preencode(&value.user_data)?;
        self.preencode(&value.tree)?;
        self.preencode(&value.hints)
    }

    fn encode(&mut self, value: &Header, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.set_byte_to_buffer(1, buffer)?; // Version
        let flags: u8 = 2 | 4; // Manifest and key pair, TODO: external=1
        self.set_byte_to_buffer(flags, buffer)?;
        self.encode_fixed_32(&value.key, buffer)?;
        self.encode(&value.manifest, buffer)?;
        self.encode(&value.key_pair, buffer)?;
        self.encode(&value.user_data, buffer)?;
        self.encode(&value.tree, buffer)?;
        self.encode(&value.hints, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<Header, EncodingError> {
        let version: u8 = self.decode_u8(buffer)?;
        if version != 1 {
            panic!("Unknown oplog version {}", version);
        }
        let _flags: u8 = self.decode_u8(buffer)?;
        let key: [u8; 32] = self
            .decode_fixed_32(buffer)?
            .to_vec()
            .try_into()
            .map_err(|_err| {
                EncodingError::new(
                    EncodingErrorKind::InvalidData,
                    "Invalid key in oplog header",
                )
            })?;
        let manifest: Manifest = self.decode(buffer)?;
        let key_pair: PartialKeypair = self.decode(buffer)?;
        let user_data: Vec<String> = self.decode(buffer)?;
        let tree: HeaderTree = self.decode(buffer)?;
        let hints: HeaderHints = self.decode(buffer)?;

        Ok(Header {
            key,
            manifest,
            key_pair,
            user_data,
            tree,
            hints,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::crypto::generate_signing_key;

    #[test]
    fn encode_partial_key_pair() -> Result<(), EncodingError> {
        let mut enc_state = State::new();
        let signing_key = generate_signing_key();
        let key_pair = PartialKeypair {
            public: signing_key.verifying_key(),
            secret: Some(signing_key),
        };
        enc_state.preencode(&key_pair)?;
        let mut buffer = enc_state.create_buffer();
        // Pub key: 1 byte for length, 32 bytes for content
        // Sec key: 1 byte for length, 64 bytes for data
        let expected_len = 1 + 32 + 1 + 64;
        assert_eq!(buffer.len(), expected_len);
        assert_eq!(enc_state.end(), expected_len);
        assert_eq!(enc_state.start(), 0);
        enc_state.encode(&key_pair, &mut buffer)?;
        let mut dec_state = State::from_buffer(&buffer);
        let key_pair_ret: PartialKeypair = dec_state.decode(&buffer)?;
        assert_eq!(key_pair.public, key_pair_ret.public);
        assert_eq!(
            key_pair.secret.unwrap().to_bytes(),
            key_pair_ret.secret.unwrap().to_bytes()
        );
        Ok(())
    }

    #[test]
    fn encode_tree() -> Result<(), EncodingError> {
        let mut enc_state = State::new();
        let tree = HeaderTree::new();
        enc_state.preencode(&tree)?;
        let mut buffer = enc_state.create_buffer();
        enc_state.encode(&tree, &mut buffer)?;
        let mut dec_state = State::from_buffer(&buffer);
        let tree_ret: HeaderTree = dec_state.decode(&buffer)?;
        assert_eq!(tree, tree_ret);
        Ok(())
    }

    #[test]
    fn encode_header() -> Result<(), EncodingError> {
        let mut enc_state = State::new();
        let signing_key = generate_signing_key();
        let signing_key = PartialKeypair {
            public: signing_key.verifying_key(),
            secret: Some(signing_key),
        };
        let header = Header::new(signing_key);
        enc_state.preencode(&header)?;
        let mut buffer = enc_state.create_buffer();
        enc_state.encode(&header, &mut buffer)?;
        let mut dec_state = State::from_buffer(&buffer);
        let header_ret: Header = dec_state.decode(&buffer)?;
        assert_eq!(header.key_pair.public, header_ret.key_pair.public);
        assert_eq!(header.tree.fork, header_ret.tree.fork);
        assert_eq!(header.tree.length, header_ret.tree.length);
        assert_eq!(header.tree.length, header_ret.tree.length);
        assert_eq!(header.manifest.hash, header_ret.manifest.hash);
        assert_eq!(
            header.manifest.signer.public_key,
            header_ret.manifest.signer.public_key
        );
        assert_eq!(
            header.manifest.signer.signature,
            header_ret.manifest.signer.signature
        );
        Ok(())
    }
}
