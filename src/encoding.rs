//! Hypercore-specific compact encodings
use crate::{
    crypto::{Manifest, ManifestSigner},
    DataBlock, DataHash, DataSeek, DataUpgrade, Node, RequestBlock, RequestSeek, RequestUpgrade,
};
use compact_encoding::{
    as_array, encode_bytes_fixed, encoded_size_usize, map_decode, map_encode, sum_encoded_size,
    take_array, write_slice, CompactEncoding, EncodingError, EncodingErrorKind, VecEncodable,
};

impl CompactEncoding for Node {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(self.index, self.length) + 32)
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        let hash = as_array::<32>(&self.hash)?;
        Ok(map_encode!(buffer, self.index, self.length, hash))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ((index, length, hash), rest) = map_decode!(buffer, [u64, u64, [u8; 32]]);
        Ok((Node::new(index, hash.to_vec(), length), rest))
    }
}

impl VecEncodable for Node {
    fn vec_encoded_size(vec: &[Self]) -> Result<usize, EncodingError>
    where
        Self: Sized,
    {
        let mut out = encoded_size_usize(vec.len());
        for x in vec {
            out += x.encoded_size()?;
        }
        Ok(out)
    }
}

impl CompactEncoding for RequestBlock {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(self.index, self.nodes))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        Ok(map_encode!(buffer, self.index, self.nodes))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ((index, nodes), rest) = map_decode!(buffer, [u64, u64]);
        Ok((RequestBlock { index, nodes }, rest))
    }
}

impl CompactEncoding for RequestSeek {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        self.bytes.encoded_size()
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        self.bytes.encode(buffer)
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let (bytes, rest) = u64::decode(buffer)?;
        Ok((RequestSeek { bytes }, rest))
    }
}

impl CompactEncoding for RequestUpgrade {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(self.start, self.length))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        Ok(map_encode!(buffer, self.start, self.length))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ((start, length), rest) = map_decode!(buffer, [u64, u64]);
        Ok((RequestUpgrade { start, length }, rest))
    }
}

impl CompactEncoding for DataBlock {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(self.index, self.value, self.nodes))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        Ok(map_encode!(buffer, self.index, self.value, self.nodes))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ((index, value, nodes), rest) = map_decode!(buffer, [u64, Vec<u8>, Vec<Node>]);
        Ok((
            DataBlock {
                index,
                value,
                nodes,
            },
            rest,
        ))
    }
}

impl CompactEncoding for DataHash {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(self.index, self.nodes))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        Ok(map_encode!(buffer, self.index, self.nodes))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ((index, nodes), rest) = map_decode!(buffer, [u64, Vec<Node>]);
        Ok((DataHash { index, nodes }, rest))
    }
}

impl CompactEncoding for DataSeek {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(self.bytes, self.nodes))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        Ok(map_encode!(buffer, self.bytes, self.nodes))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ((bytes, nodes), rest) = map_decode!(buffer, [u64, Vec<Node>]);
        Ok((DataSeek { bytes, nodes }, rest))
    }
}

// from:
// https://github.com/holepunchto/hypercore/blob/d21ebdeca1b27eb4c2232f8af17d5ae939ee97f2/lib/messages.js#L394
impl CompactEncoding for DataUpgrade {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(
            self.start,
            self.length,
            self.nodes,
            self.additional_nodes,
            self.signature
        ))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        Ok(map_encode!(
            buffer,
            self.start,
            self.length,
            self.nodes,
            self.additional_nodes,
            self.signature
        ))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ((start, length, nodes, additional_nodes, signature), rest) =
            map_decode!(buffer, [u64, u64, Vec<Node>, Vec<Node>, Vec<u8>]);
        Ok((
            DataUpgrade {
                start,
                length,
                nodes,
                additional_nodes,
                signature,
            },
            rest,
        ))
    }
}

impl CompactEncoding for ManifestSigner {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(
            1  /* Signature */ + 32  /* namespace */ + 32, /* public_key */
        )
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        let rest = if &self.signature == "ed25519" {
            write_slice(&[0], buffer)?
        } else {
            return Err(EncodingError::new(
                EncodingErrorKind::InvalidData,
                &format!("Unknown signature type: {}", &self.signature),
            ));
        };
        let rest = encode_bytes_fixed(&self.namespace, rest)?;
        encode_bytes_fixed(&self.public_key, rest)
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ([signature_id], rest) = take_array::<1>(buffer)?;
        let signature: String = if signature_id != 0 {
            return Err(EncodingError::new(
                EncodingErrorKind::InvalidData,
                &format!("Unknown signature id: {signature_id}"),
            ));
        } else {
            "ed25519".to_string()
        };

        let (namespace, rest) = take_array::<32>(rest)?;
        let (public_key, rest) = take_array::<32>(rest)?;
        Ok((
            ManifestSigner {
                signature,
                namespace,
                public_key,
            },
            rest,
        ))
    }
}

impl CompactEncoding for Manifest {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(1 // Version
        + 1 // hash in one byte
        + 1 // type in one byte
        + self.signer.encoded_size()?)
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        let rest = write_slice(&[0], buffer)?;
        let rest = if &self.hash == "blake2b" {
            write_slice(&[0], rest)?
        } else {
            return Err(EncodingError::new(
                EncodingErrorKind::InvalidData,
                &format!("Unknown hash: {}", &self.hash),
            ));
        };
        let rest = write_slice(&[1], rest)?;
        self.signer.encode(rest)
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ([version], rest) = take_array::<1>(buffer)?;
        if version != 0 {
            panic!("Unknown manifest version {}", version);
        }
        let ([hash_id], rest) = take_array::<1>(rest)?;
        let hash: String = if hash_id != 0 {
            return Err(EncodingError::new(
                EncodingErrorKind::InvalidData,
                &format!("Unknown hash id: {hash_id}"),
            ));
        } else {
            "blake2b".to_string()
        };
        let ([manifest_type], rest) = take_array::<1>(rest)?;
        if manifest_type != 1 {
            return Err(EncodingError::new(
                EncodingErrorKind::InvalidData,
                &format!("Unknown manifest type: {manifest_type}"),
            ));
        }
        let (signer, rest) = ManifestSigner::decode(rest)?;
        Ok((Manifest { hash, signer }, rest))
    }
}
