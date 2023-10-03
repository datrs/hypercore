//! Hypercore-specific compact encodings
pub use compact_encoding::{CompactEncoding, EncodingError, EncodingErrorKind, State};
use std::convert::TryInto;
use std::ops::{Deref, DerefMut};

use crate::{
    crypto::{Manifest, ManifestSigner},
    DataBlock, DataHash, DataSeek, DataUpgrade, Node, RequestBlock, RequestSeek, RequestUpgrade,
};

#[derive(Debug, Clone)]
/// Wrapper struct for compact_encoding::State
pub struct HypercoreState(pub State);

impl Default for HypercoreState {
    /// Passthrought to compact_encoding
    fn default() -> Self {
        Self::new()
    }
}

impl HypercoreState {
    /// Passthrought to compact_encoding
    pub fn new() -> HypercoreState {
        HypercoreState(State::new())
    }

    /// Passthrought to compact_encoding
    pub fn new_with_size(size: usize) -> (HypercoreState, Box<[u8]>) {
        let (state, buffer) = State::new_with_size(size);
        (HypercoreState(state), buffer)
    }

    /// Passthrought to compact_encoding
    pub fn new_with_start_and_end(start: usize, end: usize) -> HypercoreState {
        HypercoreState(State::new_with_start_and_end(start, end))
    }

    /// Passthrought to compact_encoding
    pub fn from_buffer(buffer: &[u8]) -> HypercoreState {
        HypercoreState(State::from_buffer(buffer))
    }
}

impl Deref for HypercoreState {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HypercoreState {
    fn deref_mut(&mut self) -> &mut State {
        &mut self.0
    }
}

impl CompactEncoding<Node> for HypercoreState {
    fn preencode(&mut self, value: &Node) -> Result<usize, EncodingError> {
        self.0.preencode(&value.index)?;
        self.0.preencode(&value.length)?;
        self.0.preencode_fixed_32()
    }

    fn encode(&mut self, value: &Node, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.0.encode(&value.index, buffer)?;
        self.0.encode(&value.length, buffer)?;
        self.0.encode_fixed_32(&value.hash, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<Node, EncodingError> {
        let index: u64 = self.0.decode(buffer)?;
        let length: u64 = self.0.decode(buffer)?;
        let hash: Box<[u8]> = self.0.decode_fixed_32(buffer)?;
        Ok(Node::new(index, hash.to_vec(), length))
    }
}

impl CompactEncoding<Vec<Node>> for HypercoreState {
    fn preencode(&mut self, value: &Vec<Node>) -> Result<usize, EncodingError> {
        let len = value.len();
        self.0.preencode(&len)?;
        for val in value {
            self.preencode(val)?;
        }
        Ok(self.end())
    }

    fn encode(&mut self, value: &Vec<Node>, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        let len = value.len();
        self.0.encode(&len, buffer)?;
        for val in value {
            self.encode(val, buffer)?;
        }
        Ok(self.start())
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<Vec<Node>, EncodingError> {
        let len: usize = self.0.decode(buffer)?;
        let mut value = Vec::with_capacity(len);
        for _ in 0..len {
            value.push(self.decode(buffer)?);
        }
        Ok(value)
    }
}

impl CompactEncoding<RequestBlock> for HypercoreState {
    fn preencode(&mut self, value: &RequestBlock) -> Result<usize, EncodingError> {
        self.0.preencode(&value.index)?;
        self.0.preencode(&value.nodes)
    }

    fn encode(&mut self, value: &RequestBlock, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.0.encode(&value.index, buffer)?;
        self.0.encode(&value.nodes, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<RequestBlock, EncodingError> {
        let index: u64 = self.0.decode(buffer)?;
        let nodes: u64 = self.0.decode(buffer)?;
        Ok(RequestBlock { index, nodes })
    }
}

impl CompactEncoding<RequestSeek> for HypercoreState {
    fn preencode(&mut self, value: &RequestSeek) -> Result<usize, EncodingError> {
        self.0.preencode(&value.bytes)
    }

    fn encode(&mut self, value: &RequestSeek, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.0.encode(&value.bytes, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<RequestSeek, EncodingError> {
        let bytes: u64 = self.0.decode(buffer)?;
        Ok(RequestSeek { bytes })
    }
}

impl CompactEncoding<RequestUpgrade> for HypercoreState {
    fn preencode(&mut self, value: &RequestUpgrade) -> Result<usize, EncodingError> {
        self.0.preencode(&value.start)?;
        self.0.preencode(&value.length)
    }

    fn encode(
        &mut self,
        value: &RequestUpgrade,
        buffer: &mut [u8],
    ) -> Result<usize, EncodingError> {
        self.0.encode(&value.start, buffer)?;
        self.0.encode(&value.length, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<RequestUpgrade, EncodingError> {
        let start: u64 = self.0.decode(buffer)?;
        let length: u64 = self.0.decode(buffer)?;
        Ok(RequestUpgrade { start, length })
    }
}

impl CompactEncoding<DataBlock> for HypercoreState {
    fn preencode(&mut self, value: &DataBlock) -> Result<usize, EncodingError> {
        self.0.preencode(&value.index)?;
        self.0.preencode(&value.value)?;
        self.preencode(&value.nodes)
    }

    fn encode(&mut self, value: &DataBlock, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.0.encode(&value.index, buffer)?;
        self.0.encode(&value.value, buffer)?;
        self.encode(&value.nodes, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<DataBlock, EncodingError> {
        let index: u64 = self.0.decode(buffer)?;
        let value: Vec<u8> = self.0.decode(buffer)?;
        let nodes: Vec<Node> = self.decode(buffer)?;
        Ok(DataBlock {
            index,
            value,
            nodes,
        })
    }
}

impl CompactEncoding<DataHash> for HypercoreState {
    fn preencode(&mut self, value: &DataHash) -> Result<usize, EncodingError> {
        self.0.preencode(&value.index)?;
        self.preencode(&value.nodes)
    }

    fn encode(&mut self, value: &DataHash, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.0.encode(&value.index, buffer)?;
        self.encode(&value.nodes, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<DataHash, EncodingError> {
        let index: u64 = self.0.decode(buffer)?;
        let nodes: Vec<Node> = self.decode(buffer)?;
        Ok(DataHash { index, nodes })
    }
}

impl CompactEncoding<DataSeek> for HypercoreState {
    fn preencode(&mut self, value: &DataSeek) -> Result<usize, EncodingError> {
        self.0.preencode(&value.bytes)?;
        self.preencode(&value.nodes)
    }

    fn encode(&mut self, value: &DataSeek, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.0.encode(&value.bytes, buffer)?;
        self.encode(&value.nodes, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<DataSeek, EncodingError> {
        let bytes: u64 = self.0.decode(buffer)?;
        let nodes: Vec<Node> = self.decode(buffer)?;
        Ok(DataSeek { bytes, nodes })
    }
}

impl CompactEncoding<DataUpgrade> for HypercoreState {
    fn preencode(&mut self, value: &DataUpgrade) -> Result<usize, EncodingError> {
        self.0.preencode(&value.start)?;
        self.0.preencode(&value.length)?;
        self.preencode(&value.nodes)?;
        self.preencode(&value.additional_nodes)?;
        self.0.preencode(&value.signature)
    }

    fn encode(&mut self, value: &DataUpgrade, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.0.encode(&value.start, buffer)?;
        self.0.encode(&value.length, buffer)?;
        self.encode(&value.nodes, buffer)?;
        self.encode(&value.additional_nodes, buffer)?;
        self.0.encode(&value.signature, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<DataUpgrade, EncodingError> {
        let start: u64 = self.0.decode(buffer)?;
        let length: u64 = self.0.decode(buffer)?;
        let nodes: Vec<Node> = self.decode(buffer)?;
        let additional_nodes: Vec<Node> = self.decode(buffer)?;
        let signature: Vec<u8> = self.0.decode(buffer)?;
        Ok(DataUpgrade {
            start,
            length,
            nodes,
            additional_nodes,
            signature,
        })
    }
}

impl CompactEncoding<Manifest> for State {
    fn preencode(&mut self, value: &Manifest) -> Result<usize, EncodingError> {
        self.add_end(1)?; // Version
        self.add_end(1)?; // hash in one byte
        self.add_end(1)?; // type in one byte
        self.preencode(&value.signer)
    }

    fn encode(&mut self, value: &Manifest, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        self.set_byte_to_buffer(0, buffer)?; // Version
        if &value.hash == "blake2b" {
            self.set_byte_to_buffer(0, buffer)?; // Version
        } else {
            return Err(EncodingError::new(
                EncodingErrorKind::InvalidData,
                &format!("Unknown hash: {}", &value.hash),
            ));
        }
        // Type. 0: static, 1: signer, 2: multiple signers
        self.set_byte_to_buffer(1, buffer)?; // Version
        self.encode(&value.signer, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<Manifest, EncodingError> {
        let version: u8 = self.decode_u8(buffer)?;
        if version != 0 {
            panic!("Unknown manifest version {}", version);
        }
        let hash_id: u8 = self.decode_u8(buffer)?;
        let hash: String = if hash_id != 0 {
            return Err(EncodingError::new(
                EncodingErrorKind::InvalidData,
                &format!("Unknown hash id: {hash_id}"),
            ));
        } else {
            "blake2b".to_string()
        };

        let manifest_type: u8 = self.decode_u8(buffer)?;
        if manifest_type != 1 {
            return Err(EncodingError::new(
                EncodingErrorKind::InvalidData,
                &format!("Unknown manifest type: {manifest_type}"),
            ));
        }
        let signer: ManifestSigner = self.decode(buffer)?;

        Ok(Manifest { hash, signer })
    }
}

impl CompactEncoding<ManifestSigner> for State {
    fn preencode(&mut self, _value: &ManifestSigner) -> Result<usize, EncodingError> {
        self.add_end(1)?; // Signature
        self.preencode_fixed_32()?;
        self.preencode_fixed_32()
    }

    fn encode(
        &mut self,
        value: &ManifestSigner,
        buffer: &mut [u8],
    ) -> Result<usize, EncodingError> {
        if &value.signature == "ed25519" {
            self.set_byte_to_buffer(0, buffer)?;
        } else {
            return Err(EncodingError::new(
                EncodingErrorKind::InvalidData,
                &format!("Unknown signature type: {}", &value.signature),
            ));
        }
        self.encode_fixed_32(&value.namespace, buffer)?;
        self.encode_fixed_32(&value.public_key, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<ManifestSigner, EncodingError> {
        let signature_id: u8 = self.decode_u8(buffer)?;
        let signature: String = if signature_id != 0 {
            return Err(EncodingError::new(
                EncodingErrorKind::InvalidData,
                &format!("Unknown signature id: {signature_id}"),
            ));
        } else {
            "ed25519".to_string()
        };
        let namespace: [u8; 32] =
            self.decode_fixed_32(buffer)?
                .to_vec()
                .try_into()
                .map_err(|_err| {
                    EncodingError::new(
                        EncodingErrorKind::InvalidData,
                        "Invalid namespace in manifest signer",
                    )
                })?;
        let public_key: [u8; 32] =
            self.decode_fixed_32(buffer)?
                .to_vec()
                .try_into()
                .map_err(|_err| {
                    EncodingError::new(
                        EncodingErrorKind::InvalidData,
                        "Invalid public key in manifest signer",
                    )
                })?;

        Ok(ManifestSigner {
            signature,
            namespace,
            public_key,
        })
    }
}
