use crate::bitcoin::Encodable;
use crate::Error;
use bytes::{Buf, BufMut};
use hex::{FromHex, ToHex};
use ring::digest::{digest, SHA256};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;

/// A struct representing a hash, specifically a SHA256d hash.
///
/// This is the hash type that is generally used within the Bitcoin infrastructure.
///
/// Note that [TxHash], [BlockHash], and [MerkleRoot] are all type aliases for [struct@Hash]. Those aliases
/// should generally be used instead of this struct.
/// [MerkleRoot]: crate::bitcoin::MerkleRoot
/// [TxHash]: crate::bitcoin::TxHash
/// [BlockHash]: crate::bitcoin::BlockHash
// We're not going to use a Bytes here. https://docs.rs/bytes/latest/bytes/struct.Bytes.html# reports
// that Bytes struct has 4 x usize fields = 32 bytes (on 64-bit architecture, our main goal). This is
// equal in size to the hash, might as well just copy it when needed.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash {
    pub raw: [u8; 32],
}

impl Hash {
    pub const SIZE: u64 = 32;
    pub const HEX_SIZE: u64 = Hash::SIZE * 2;
    pub const ZERO: Hash = Hash {
        raw: [0; Self::SIZE as usize],
    };

    /// Double SHA256 hash the given data.
    pub fn sha256d(data: &[u8]) -> Hash {
        let sha256 = digest(&SHA256, data);
        let sha256d = digest(&SHA256, sha256.as_ref());
        let mut hash256 = [0; 32];
        hash256.clone_from_slice(sha256d.as_ref());
        Hash { raw: hash256 }
    }
    
    pub fn from_slice(slice: &[u8]) -> Hash {
        let mut hash = [0; 32];
        hash.copy_from_slice(slice);
        Hash { raw: hash }
    }

    // helper for ToHex trait implementation
    fn generic_encode_hex<T, F>(&self, mut encode_fn: F) -> T
    where
        T: FromIterator<char>,
        F: FnMut(&[u8]) -> String,
    {
        let mut reversed_bytes = self.raw;
        reversed_bytes.reverse();
        encode_fn(&reversed_bytes).chars().collect()
    }
}

impl Encodable for Hash {
    fn from_binary(buffer: &mut dyn Buf) -> crate::Result<Self>
    where
        Self: Sized,
    {
        if buffer.remaining() < Self::SIZE as usize {
            Err(Error::DataTooSmall)
        } else {
            let mut hash = [0; 32];
            buffer.copy_to_slice(&mut hash);
            Ok(Self { raw: hash })
        }
    }

    fn to_binary(&self, buffer: &mut dyn BufMut) -> crate::Result<()> {
        Ok(buffer.put_slice(&self.raw))
    }

    fn encoded_size(&self) -> u64 {
        Self::SIZE
    }
}

impl FromHex for Hash {
    type Error = Error;

    /// Converts a string of 64 hex characters into a hash. The bytes of the hex encoded form are reversed in
    /// accordance with Bitcoin standards.
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let hex = hex.as_ref();
        if hex.len() != Hash::HEX_SIZE as usize {
            let msg = format!(
                "Length of hex encoded hash must be 64. Len is {:}.",
                hex.len()
            );
            return Err(Error::BadArgument(msg));
        }
        match hex::decode(hex) {
            Ok(mut hash_bytes) => {
                // Reverse bytes in place to match Bitcoin standard representation.
                hash_bytes.reverse();
                let mut hash_array = [0u8; Hash::SIZE as usize];
                hash_array.copy_from_slice(&hash_bytes);
                Ok(Hash { raw: hash_array })
            }
            Err(e) => Err(Error::FromHexError(e)),
        }
    }
}

impl ToHex for Hash {
    /// Converts the hash into a hex string. The bytes are reversed in the hex string in accordance with
    /// Bitcoin standard representation.
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        self.generic_encode_hex(|bytes| hex::encode(bytes))
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        self.generic_encode_hex(|bytes| hex::encode_upper(bytes))
    }
}

impl From<&[u8]> for Hash {
    /// This converts a u8 encoded hash into a Hash struct.
    fn from(hash_as_bytes: &[u8]) -> Hash {
        Hash {
            raw: <[u8; 32]>::try_from(hash_as_bytes).expect("Hash must be 32 bytes"),
        }
    }
}

impl From<[u8; 32]> for Hash {
    fn from(value: [u8; 32]) -> Self {
        Hash { raw: value }
    }
}

impl From<Hash> for [u8; 32] {
    /// Convert from Hash to u8 encoding
    fn from(value: Hash) -> Self {
        value.raw
    }
}

impl From<Hash> for Vec<u8> {
    fn from(value: Hash) -> Self {
        value.raw.to_vec()
    }
}

impl From<&str> for Hash {
    /// This converts a hex encoded hash into a Hash struct.
    fn from(hash_as_hex: &str) -> Hash {
        Hash::from_hex(hash_as_hex).unwrap()
    }
}

impl Ord for Hash {
    /// Define the ordering of hashes. The order direction matches the alphabetic ordering of
    /// their hex representations.
    ///
    /// The ordering is byte-wise from the last byte to the first of the encoded form. This matches
    /// the alphabetical ordering of the hex representation since the hex representation is  
    /// reversed byte-wise.
    ///
    /// I'm not entirely sure this is the right thing to do. The good thing is that it will match
    /// what a human will want to see in an ordered list of hashes. It also seems to be what other
    /// libraries do. However, it may mismatch with other systems. For example, if storing the hashes
    /// in binary form in a database field, the database may order them differently.
    fn cmp(&self, other: &Hash) -> Ordering {
        for i in (0..32).rev() {
            if self.raw[i] < other.raw[i] {
                return Ordering::Less;
            } else if self.raw[i] > other.raw[i] {
                return Ordering::Greater;
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for Hash {
    fn partial_cmp(&self, other: &Hash) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.encode_hex::<String>())
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.encode_hex::<String>())
    }
}

impl Serialize for Hash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.encode_hex::<String>().as_ref())
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Hash::from_hex(s).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hex;

    #[test]
    fn sha256d_test() {
        let x = hex::decode("0123456789abcdef").unwrap();
        let e = hex::encode(Hash::sha256d(&x).raw);
        assert_eq!(
            e,
            "137ad663f79da06e282ed0abbec4d70523ced5ff8e39d5c2e5641d978c5925aa"
        );
    }

    #[test]
    fn hash_decode() {
        // Valid
        let s1 = "0000000000000000000000000000000000000000000000000000000000000000";
        let s2 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let s3 = "abcdef0000112233445566778899abcdef000011223344556677889912345678";
        assert!(Hash::from_hex(s1).is_ok());
        assert!(Hash::from_hex(s2).is_ok());
        assert!(Hash::from_hex(s3).is_ok());

        // Invalid
        let s1 = "000000000000000000000000000000000000000000000000000000000000000";
        let s2 = "00000000000000000000000000000000000000000000000000000000000000000";
        let s3 = "000000000000000000000000000000000000000000000000000000000000000g";
        assert!(Hash::from_hex(s1).is_err());
        assert!(Hash::from_hex(s2).is_err());
        assert!(Hash::from_hex(s3).is_err());
    }

    #[test]
    fn hash_compare() {
        let s1 = "5555555555555555555555555555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert_eq!(Hash::from_hex(s1).unwrap(), Hash::from_hex(s2).unwrap());

        let s1 = "0555555555555555555555555555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash::from_hex(s1).unwrap() < Hash::from_hex(s2).unwrap());

        let s1 = "5555555555555555555555555555555555555555555555555555555555555550";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash::from_hex(s1).unwrap() < Hash::from_hex(s2).unwrap());

        let s1 = "6555555555555555555555555555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash::from_hex(s1).unwrap() > Hash::from_hex(s2).unwrap());

        let s1 = "5555555555555555555555555555555555555555555555555555555555555556";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash::from_hex(s1).unwrap() > Hash::from_hex(s2).unwrap());
    }

    /// Test binary read of hash
    #[test]
    fn hash_read() {
        let mut b = Bytes::from(vec![
            0xbeu8, 0xc7, 0x7b, 0x08, 0x3c, 0xf7, 0xb7, 0x5c, 0x97, 0xcc, 0xfa, 0x0c, 0x4b, 0x0c,
            0x0c, 0x40, 0xa6, 0xe5, 0xae, 0x6b, 0x05, 0xab, 0x12, 0xc9, 0x38, 0x81, 0xaf, 0x7f,
            0x8a, 0x04, 0x53, 0xf2,
        ]);
        let h = Hash::from_binary(&mut b).unwrap();
        assert_eq!(
            h.encode_hex::<String>(),
            "f253048a7faf8138c912ab056baee5a6400c0c4b0cfacc975cb7f73c087bc7be"
        );
    }

    #[test]
    fn hash_write() {
        let s = "684b2f7e73dec228a7bf9a73495eeb6a28f2cda66b7f8e1627fdff8922ec754f";
        let h = Hash::from_hex(s).unwrap();
        let mut v = Vec::with_capacity(Hash::SIZE as usize);
        h.to_binary(&mut v).unwrap();
        let c = vec![
            0x4f, 0x75, 0xec, 0x22, 0x89, 0xff, 0xfd, 0x27, 0x16, 0x8e, 0x7f, 0x6b, 0xa6, 0xcd,
            0xf2, 0x28, 0x6a, 0xeb, 0x5e, 0x49, 0x73, 0x9a, 0xbf, 0xa7, 0x28, 0xc2, 0xde, 0x73,
            0x7e, 0x2f, 0x4b, 0x68,
        ];
        assert_eq!(v, c);
    }

    #[test]
    fn json_serialize_hash() {
        let hash =
            Hash::from_hex("0000000000000000069347185643c805ff7e00fae025316393e34fa67274df4e")
                .expect("Failed to decode test hash");
        let serialized = serde_json::to_string(&hash).expect("Failed to serialize");
        // Ensure it serializes to a hex string
        assert_eq!(
            serialized,
            "\"0000000000000000069347185643c805ff7e00fae025316393e34fa67274df4e\""
        );
    }

    #[test]
    fn json_deserialize_hash() {
        let original_hash = Hash::sha256d(b"hello world");
        let serialized = serde_json::to_string(&original_hash).expect("Failed to serialize");
        let deserialized: Hash = serde_json::from_str(&serialized).expect("Failed to deserialize");
        // Ensure the deserialized hash matches the original
        assert_eq!(deserialized, original_hash);
    }
}
