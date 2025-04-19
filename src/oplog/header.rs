use compact_encoding::{decode_usize, take_array, write_array, CompactEncoding, EncodingError};
use ed25519_dalek::{SigningKey, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};

use crate::crypto::default_signer_manifest;
use crate::crypto::Manifest;
use crate::{chain_encoded_bytes, VerifyingKey};
use crate::{sum_encoded_size, PartialKeypair};

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

#[macro_export]
/// Helper for decoding a struct with compact encodable
macro_rules! decode {
    // Match the pattern: decode!(StructName, buffer, {field1: type1, field2: type2, ...})
    ($struct_name:ident, $buffer:expr, {
        $($field_name:ident : $field_type:ty),* $(,)?
    }) => {{

        // Variable to hold the current buffer state
        let mut current_buffer = $buffer;

        // Decode each field in sequence
        $(
            let ($field_name, new_buffer) = <$field_type>::decode(current_buffer)?;
            current_buffer = new_buffer;
        )*

        // Create the struct with decoded fields
        let result = $struct_name {
            $(
                $field_name,
            )*
        };

        // Return the struct and the remaining buffer
        Ok((result, current_buffer))
    }};
 }

impl CompactEncoding for HeaderTree {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(self, fork, length, root_hash, signature))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        Ok(chain_encoded_bytes!(
            self, buffer, fork, length, root_hash, signature
        ))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        decode!(HeaderTree, buffer, {fork: u64, length: u64, root_hash: Box<[u8]>, signature: Box<[u8]>})
    }
}

/// NB: In Javascript's sodium the secret key contains in itself also the public key, so to
/// maintain binary compatibility, we store the public key in the oplog now twice.
impl CompactEncoding for PartialKeypair {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(1 // len of public key 
            + PUBLIC_KEY_LENGTH // public key bytes
            + match self.secret {
            // Secret key contains the public key
            Some(_) => 1 + SECRET_KEY_LENGTH + PUBLIC_KEY_LENGTH,
            None => 1,
        })
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        let public_key = self.public.as_bytes().to_vec();
        let rest = public_key.encode(buffer)?;
        match &self.secret {
            Some(sk) => {
                let sk_bytes = [&sk.to_bytes()[..], &public_key[..]].concat();
                sk_bytes.encode(rest)
            }
            None => write_array(&[0], rest),
        }
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        // the ful secret/private key contains the public key duplicated in it
        const FULL_SIGNING_KEY_LENGTH: usize = SECRET_KEY_LENGTH + PUBLIC_KEY_LENGTH;
        let (pk_len, rest) = decode_usize(buffer)?;
        let (public, rest) = match pk_len {
            PUBLIC_KEY_LENGTH => {
                let (pk_bytes, rest) = take_array::<PUBLIC_KEY_LENGTH>(rest)?;
                let public = VerifyingKey::from_bytes(&pk_bytes).map_err(|e| {
                    EncodingError::invalid_data(&format!(
                        "Could not decode public key. error: [{e}]"
                    ))
                })?;
                (public, rest)
            }
            len => {
                return Err(EncodingError::invalid_data(&format!(
                    "Incorrect public key length while decoding. length = [{len}] expected [{PUBLIC_KEY_LENGTH}]"
                )))
            }
        };
        let (sk_len, rest) = decode_usize(rest)?;
        let (secret, rest) = match sk_len {
            0 => (None, rest),
            // full signing key = secret_key.cocat(public_key)
            FULL_SIGNING_KEY_LENGTH => {
                let (full_key_bytes, rest) = take_array::<FULL_SIGNING_KEY_LENGTH>(rest)?;
                let (sk_bytes, _pk_bytes) = take_array::<SECRET_KEY_LENGTH>(&full_key_bytes)?;
                (Some(SigningKey::from_bytes(&sk_bytes)), rest)
            }
            len => {
                return Err(EncodingError::invalid_data(&format!(
                    "Incorrect secret key length while decoding. length = [{len}] expected [{FULL_SIGNING_KEY_LENGTH}]"
                )))
            }
        };
        Ok((PartialKeypair { public, secret }, rest))
    }
}

/// Oplog header hints
#[derive(Debug, Clone)]
pub(crate) struct HeaderHints {
    pub(crate) reorgs: Vec<String>,
    pub(crate) contiguous_length: u64,
}

impl CompactEncoding for HeaderHints {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(self, reorgs, contiguous_length))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        Ok(chain_encoded_bytes!(
            self,
            buffer,
            reorgs,
            contiguous_length
        ))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        decode!(HeaderHints, buffer, {reorgs: Vec<String>, contiguous_length: u64 })
    }
}

impl CompactEncoding for Header {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(1 + 1 + 32 + sum_encoded_size!(self, manifest, key_pair, user_data, tree, hints))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        let rest = write_array(&[1, 2 | 4], buffer)?;
        let rest = self.key.encode(rest)?;
        let rest = self.manifest.encode(rest)?;
        let rest = self.key_pair.encode(rest)?;
        let rest = self.user_data.encode(rest)?;
        let rest = self.tree.encode(rest)?;
        let rest = self.hints.encode(rest)?;
        Ok(rest)
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ([_version, _flags], rest) = take_array::<2>(buffer)?;
        let (key, rest) = take_array::<32>(rest)?;
        let (manifest, rest) = Manifest::decode(rest)?;
        let (key_pair, rest) = PartialKeypair::decode(rest)?;
        let (user_data, rest) = <Vec<String>>::decode(rest)?;
        let (tree, rest) = HeaderTree::decode(rest)?;
        let (hints, rest) = HeaderHints::decode(rest)?;
        Ok((
            Header {
                key,
                manifest,
                key_pair,
                user_data,
                tree,
                hints,
            },
            rest,
        ))
    }
}

#[cfg(test)]
mod tests {
    use compact_encoding::{map_decode, to_encoded_bytes};

    use super::*;

    use crate::crypto::generate_signing_key;

    #[test]
    fn encode_partial_key_pair() -> Result<(), EncodingError> {
        let signing_key = generate_signing_key();
        let key_pair = PartialKeypair {
            public: signing_key.verifying_key(),
            secret: Some(signing_key),
        };

        // sizeof(pk.len()) + sizeof(pk) + sizeof(sk.len() + sizeof(sk)
        let expected_len = 1 + 32 + 1 + 64;
        let encoded = to_encoded_bytes!(&key_pair);
        assert_eq!(encoded.len(), expected_len);
        let ((dec_kp,), rest) = map_decode!(&encoded, [PartialKeypair]);
        dbg!(rest);
        assert!(rest.is_empty());
        assert_eq!(key_pair.public, dec_kp.public);
        assert_eq!(
            key_pair.secret.unwrap().to_bytes(),
            dec_kp.secret.unwrap().to_bytes()
        );
        Ok(())
    }

    #[test]
    fn encode_tree() -> Result<(), EncodingError> {
        let tree = HeaderTree::new();
        let encoded = to_encoded_bytes!(tree);
        // all sizeof(0) + sizeof(0) + sizeof(vec![]) + sizeof(vec![]) == 4
        assert_eq!(encoded.len(), 4);
        let ((dec_tree,), rest) = map_decode!(&encoded, [HeaderTree]);
        assert!(rest.is_empty());
        assert_eq!(dec_tree, tree);
        Ok(())
    }

    #[test]
    fn encode_tree_with_data() -> Result<(), EncodingError> {
        let tree = HeaderTree {
            fork: 520,
            length: 647,
            root_hash: vec![12; 464].into_boxed_slice(),
            signature: vec![46; 22].into_boxed_slice(),
        };
        let encoded = to_encoded_bytes!(&tree);
        let ((dec_tree,), rest) = map_decode!(&encoded, [HeaderTree]);
        assert!(rest.is_empty());
        assert_eq!(dec_tree, tree);
        Ok(())
    }

    #[test]
    fn encode_header() -> Result<(), EncodingError> {
        //let mut enc_state = State::new();
        let signing_key = generate_signing_key();
        let signing_key = PartialKeypair {
            public: signing_key.verifying_key(),
            secret: Some(signing_key),
        };
        let header = Header::new(signing_key);
        let encoded = to_encoded_bytes!(&header);
        let ((dec_header,), rest) = map_decode!(&encoded, [Header]);
        assert!(rest.is_empty());
        assert_eq!(header.key_pair.public, dec_header.key_pair.public);
        assert_eq!(header.tree.fork, dec_header.tree.fork);
        assert_eq!(header.tree.length, dec_header.tree.length);
        assert_eq!(header.tree.length, dec_header.tree.length);
        assert_eq!(header.manifest.hash, dec_header.manifest.hash);
        assert_eq!(
            header.manifest.signer.public_key,
            dec_header.manifest.signer.public_key
        );
        assert_eq!(
            header.manifest.signer.signature,
            dec_header.manifest.signer.signature
        );
        Ok(())
    }
}
