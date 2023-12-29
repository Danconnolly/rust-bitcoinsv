use std::cmp::Ordering;
use std::fmt;
use ring::digest::{digest, SHA256};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
// use crate::util::{Error, Result};

/// The hash that is most often used in Bitcoin is the double SHA-256 hash.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash{
    pub hash: [u8; 32],
}

impl Hash {
    /// Converts the hash into a hex string. The bytes are reversed in the hex string in accordance with
    /// Bitcoin standard representation.
    pub fn encode(&self) -> String {
        let mut r = self.hash.clone();
        r.reverse();
        hex::encode(r)
    }

    /// Converts a string of 64 hex characters into a hash. The bytes of the hex encoded form are reversed in
    /// accordance with Bitcoin standards.
    pub fn decode(s: &str) -> crate::Result<Hash> {
        if s.len() != 64 {
            let msg = format!("Length of hex encoded hash must be 64. Len of {:?} is {}", s.len(), s);
            return Err(crate::Error::BadArgument(msg));
        }
        let decoded_bytes = hex::decode(s)?;
        let mut hash_bytes = [0; 32];
        hash_bytes.clone_from_slice(&decoded_bytes);
        hash_bytes.reverse();
        Ok(Hash{hash: hash_bytes})
    }

    pub fn sha256d(data: &[u8]) -> Hash {
        let sha256 = digest(&SHA256, &data);
        let sha256d = digest(&SHA256, sha256.as_ref());
        let mut hash256 = [0; 32];
        hash256.clone_from_slice(sha256d.as_ref());
        Hash{hash: hash256}
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
        Hash::decode(hash_as_hex).unwrap()
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

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl Serialize for Hash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.encode())
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        Hash::decode(&s).map_err(|e| serde::de::Error::custom(e.to_string()))
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
        assert!(Hash::decode(s1).is_ok());
        assert!(Hash::decode(s2).is_ok());
        assert!(Hash::decode(s3).is_ok());

        // Invalid
        let s1 = "000000000000000000000000000000000000000000000000000000000000000";
        let s2 = "00000000000000000000000000000000000000000000000000000000000000000";
        let s3 = "000000000000000000000000000000000000000000000000000000000000000g";
        assert!(Hash::decode(s1).is_err());
        assert!(Hash::decode(s2).is_err());
        assert!(Hash::decode(s3).is_err());
    }

    #[test]
    fn hash_compare() {
        let s1 = "5555555555555555555555555555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert_eq!(Hash::decode(s1).unwrap(), Hash::decode(s2).unwrap());

        let s1 = "0555555555555555555555555555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash::decode(s1).unwrap() < Hash::decode(s2).unwrap());

        let s1 = "5555555555555555555555555555555555555555555555555555555555555550";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash::decode(s1).unwrap() < Hash::decode(s2).unwrap());

        let s1 = "6555555555555555555555555555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash::decode(s1).unwrap() > Hash::decode(s2).unwrap());

        let s1 = "5555555555555555555555555555555555555555555555555555555555555556";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash::decode(s1).unwrap() > Hash::decode(s2).unwrap());
    }

    #[test]
    fn json_serialize_hash() {
        let hash = Hash::decode("0000000000000000069347185643c805ff7e00fae025316393e34fa67274df4e").expect("Failed to decode test hash");
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