use std::cmp::Ordering;
use std::fmt;
use ring::digest::{digest, SHA256};
use crate::util::{Error, Result};

/// The hash that is most often used in Bitcoin is the double SHA-256 hash.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash{
    pub hash: [u8; 32],
}

impl Hash {
    /// Converts the hash into a hex string
    pub fn encode(&self) -> String {
        let mut r = self.hash.clone();
        r.reverse();
        hex::encode(r)
    }

    /// Converts a string of 64 hex characters into a hash
    pub fn decode(s: &str) -> Result<Hash> {
        let decoded_bytes = hex::decode(s)?;
        if decoded_bytes.len() != 32 {
            let msg = format!("Length {} of {:?}", decoded_bytes.len(), decoded_bytes);
            return Err(Error::BadArgument(msg));
        }
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
    fn from(data: &[u8]) -> Hash {
        Hash::sha256d(data)
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
}