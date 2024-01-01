use std::cmp::Ordering;
use std::fmt;
use async_trait::async_trait;
use hex::{FromHex, ToHex};
use ring::digest::{digest, SHA256};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::binary::Encodable;

/// The hash that is most often used in Bitcoin is the double SHA-256 hash.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash{
    pub hash: [u8; 32],
}

impl Hash {
    pub const BINARY_SIZE: usize = 32;
    pub const HEX_SIZE: usize = Hash::BINARY_SIZE * 2;

    pub fn sha256d(data: &[u8]) -> Hash {
        let sha256 = digest(&SHA256, data);
        let sha256d = digest(&SHA256, sha256.as_ref());
        let mut hash256 = [0; 32];
        hash256.clone_from_slice(sha256d.as_ref());
        Hash{hash: hash256}
    }

    // helper for ToHex trait implementation
    fn generic_encode_hex<T, F>(&self, mut encode_fn: F) -> T
        where
            T: FromIterator<char>,
            F: FnMut(&[u8]) -> String,
    {
        let mut reversed_bytes = self.hash;
        reversed_bytes.reverse();
        encode_fn(&reversed_bytes).chars().collect()
    }
}

#[async_trait]
impl Encodable for Hash {
    async fn read<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Hash> {
        let mut hash_value: [u8; 32] = [0; 32];
        let _bytes_read = reader.read_exact(&mut hash_value).await?;
        return Ok(Hash {
            hash: hash_value,
        });
    }

    async fn write<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_all(&self.hash).await?;
        Ok(())
    }
}

impl FromHex for Hash {
    type Error = crate::Error;

    /// Converts a string of 64 hex characters into a hash. The bytes of the hex encoded form are reversed in
    /// accordance with Bitcoin standards.
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let hex = hex.as_ref();
        if hex.len() != Hash::HEX_SIZE {
            let msg = format!("Length of hex encoded hash must be 64. Len is {:}.", hex.len());
            return Err(crate::Error::BadArgument(msg));
        }
        match hex::decode(hex) {
            Ok(mut hash_bytes) => {
                // Reverse bytes in place to match Bitcoin standard representation.
                hash_bytes.reverse();
                let mut hash_array = [0u8; Hash::BINARY_SIZE];
                hash_array.copy_from_slice(&hash_bytes);
                Ok(Hash { hash: hash_array })
            },
            Err(e) => Err(crate::Error::FromHexError(e)),
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
            hash: <[u8; 32]>::try_from(hash_as_bytes).expect("Hash must be 32 bytes"),
        }
    }
}

impl From<&str> for Hash {
    /// This converts a hex encoded hash into a Hash struct.
    fn from(hash_as_hex: &str) -> Hash {
        Hash::from_hex(hash_as_hex).unwrap()
    }
}

impl Ord for Hash {
    fn cmp(&self, other: &Hash) -> Ordering {
        for i in (0..32).rev() {
            if self.hash[i] < other.hash[i] {
                return Ordering::Less;
            } else if self.hash[i] > other.hash[i] {
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        Hash::from_hex(&s).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    #[test]
    fn sha256d_test() {
        let x = hex::decode("0123456789abcdef").unwrap();
        let e = hex::encode(Hash::sha256d(&x).hash);
        assert_eq!(e, "137ad663f79da06e282ed0abbec4d70523ced5ff8e39d5c2e5641d978c5925aa");
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
    #[tokio::test]
    async fn hash_read() {
        let b = vec![
            0xbe, 0xc7, 0x7b, 0x08, 0x3c, 0xf7, 0xb7, 0x5c,
            0x97, 0xcc, 0xfa, 0x0c, 0x4b, 0x0c, 0x0c, 0x40,
            0xa6, 0xe5, 0xae, 0x6b, 0x05, 0xab, 0x12, 0xc9,
            0x38, 0x81, 0xaf, 0x7f, 0x8a, 0x04, 0x53, 0xf2
        ];
        let h = Hash::read(&mut &b[..]).await.unwrap();
        assert_eq!(h.encode_hex::<String>(), "f253048a7faf8138c912ab056baee5a6400c0c4b0cfacc975cb7f73c087bc7be");
    }

    #[tokio::test]
    async fn hash_write() {
        let s = "684b2f7e73dec228a7bf9a73495eeb6a28f2cda66b7f8e1627fdff8922ec754f";
        let h = Hash::from_hex(s).unwrap();
        let mut b = Vec::new();
        h.write(&mut b).await.unwrap();
        let c = vec![
            0x4f, 0x75, 0xec, 0x22, 0x89, 0xff, 0xfd, 0x27,
            0x16, 0x8e, 0x7f, 0x6b, 0xa6, 0xcd, 0xf2, 0x28,
            0x6a, 0xeb, 0x5e, 0x49, 0x73, 0x9a, 0xbf, 0xa7,
            0x28, 0xc2, 0xde, 0x73, 0x7e, 0x2f, 0x4b, 0x68
        ];
        assert_eq!(b, c);
    }

    #[test]
    fn json_serialize_hash() {
        let hash = Hash::from_hex("0000000000000000069347185643c805ff7e00fae025316393e34fa67274df4e").expect("Failed to decode test hash");
        let serialized = serde_json::to_string(&hash).expect("Failed to serialize");
        // Ensure it serializes to a hex string
        assert_eq!(serialized, "\"0000000000000000069347185643c805ff7e00fae025316393e34fa67274df4e\"");
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