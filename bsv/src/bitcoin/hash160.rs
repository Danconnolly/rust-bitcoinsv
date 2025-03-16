use crate::bitcoin::crypto::PublicKey;
#[cfg(feature="dev_tokio")]
use crate::bitcoin::AsyncEncodable;
#[cfg(feature="dev_tokio")]
use async_trait::async_trait;
use hex::{FromHex, ToHex};
use ring::digest::{digest, SHA256};
use ripemd::digest::Update;
use ripemd::{Digest, Ripemd160};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
#[cfg(feature="dev_tokio")]
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// A 160-bit hash, specifically the RIPEMD160(SHA256) hash.
///
/// This is the hash type that is generally used for Bitcoin addresses.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash160 {
    pub hash: [u8; Self::SIZE],
}

impl Hash160 {
    pub const SIZE: usize = 20;
    pub const HEX_SIZE: usize = Hash160::SIZE * 2;
    pub const ZERO: Hash160 = Hash160 {
        hash: [0; Self::SIZE],
    };

    /// Generate the hash from the given data.
    pub fn generate(data: &[u8]) -> Hash160 {
        let sha256 = digest(&SHA256, data);
        let mut r_hasher = Ripemd160::new();
        Update::update(&mut r_hasher, sha256.as_ref());
        let ripemd = r_hasher.finalize();
        let mut hash = [0; Self::SIZE];
        hash.clone_from_slice(ripemd.as_ref());
        Hash160 { hash }
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

#[cfg(feature="dev_tokio")]
#[async_trait]
impl AsyncEncodable for Hash160 {
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let mut hash_value: [u8; Self::SIZE] = [0; Self::SIZE];
        reader.read_exact(&mut hash_value).await?;
        Ok(Hash160 { hash: hash_value })
    }

    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> crate::Result<()> {
        writer.write_all(&self.hash).await?;
        Ok(())
    }
}

impl FromHex for Hash160 {
    type Error = crate::Error;

    /// Converts a string of 40 hex characters into a hash160. The bytes of the hex encoded form are reversed in
    /// accordance with Bitcoin standards.
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let hex = hex.as_ref();
        if hex.len() != Self::HEX_SIZE {
            let msg = format!(
                "Length of hex encoded hash must be {}. Len is {:}.",
                Self::SIZE,
                hex.len()
            );
            return Err(crate::Error::BadArgument(msg));
        }
        match hex::decode(hex) {
            Ok(mut hash_bytes) => {
                // Reverse bytes in place to match Bitcoin standard representation.
                hash_bytes.reverse();
                let mut hash_array = [0u8; Self::SIZE];
                hash_array.copy_from_slice(&hash_bytes);
                Ok(Self { hash: hash_array })
            }
            Err(e) => Err(crate::Error::FromHexError(e)),
        }
    }
}

impl ToHex for Hash160 {
    /// Converts the hash into a hex string. The bytes are reversed in the hex string in accordance with
    /// Bitcoin standard representation.
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        self.generic_encode_hex(|bytes| hex::encode(bytes))
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        self.generic_encode_hex(|bytes| hex::encode_upper(bytes))
    }
}

impl From<&[u8]> for Hash160 {
    /// This converts a u8 encoded hash into a Hash160.
    fn from(hash_as_bytes: &[u8]) -> Self {
        Self {
            hash: <[u8; Self::SIZE]>::try_from(hash_as_bytes).expect("Hash must be 20 bytes"),
        }
    }
}

impl From<Hash160> for [u8; 20] {
    /// Convert from Hash to u8 encoding
    fn from(value: Hash160) -> Self {
        value.hash
    }
}

impl From<&str> for Hash160 {
    /// This converts a hex encoded hash into a Hash160.
    fn from(hash_as_hex: &str) -> Self {
        Self::from_hex(hash_as_hex).unwrap()
    }
}

impl From<PublicKey> for Hash160 {
    /// Hash a [PublicKey]. Used to produce a Bitcoin address.
    fn from(value: PublicKey) -> Self {
        value.pubkey_hash()
    }
}

impl Ord for Hash160 {
    fn cmp(&self, other: &Self) -> Ordering {
        for i in (0..Self::SIZE).rev() {
            if self.hash[i] < other.hash[i] {
                return Ordering::Less;
            } else if self.hash[i] > other.hash[i] {
                return Ordering::Greater;
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for Hash160 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Hash160 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.encode_hex::<String>())
    }
}

impl fmt::Debug for Hash160 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.encode_hex::<String>())
    }
}

impl Serialize for Hash160 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.encode_hex::<String>().as_ref())
    }
}

impl<'de> Deserialize<'de> for Hash160 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(s).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature="dev_tokio")]
    use crate::bitcoin::AsyncEncodable;
    use hex;
    use hex::FromHex;

    #[test]
    fn generate_test() {
        // tx a18fb9948823e7999a1b37f64a8ea0d83d1e5a97d121e5c65d3131d5f046806a, input 0
        // spends tx e9e64e079bf24aa6b328145d3c521123bb22964b8d530f0998d7faba2beb06b8, output 0
        // the hash160 value expected is 4cc77f98b35c178e1587747a03aaeb6932daee0b
        // the pub key provided is 02792790606e454a01e6c27372927dca961c025d25d989aeeb4b21dc2e196d2b5e
        let pubkey =
            hex::decode("02792790606e454a01e6c27372927dca961c025d25d989aeeb4b21dc2e196d2b5e")
                .unwrap();
        let e = hex::encode(Hash160::generate(&pubkey).hash);
        assert_eq!(e, "4cc77f98b35c178e1587747a03aaeb6932daee0b");
    }

    #[test]
    fn hash_decode() {
        // Valid
        let s1 = "0000000000000000000000000000000000000000";
        let s2 = "ffffffffffffffffffffffffffffffffffffffff";
        let s3 = "abcdef0000112233445566778899abcdef000011";
        assert!(Hash160::from_hex(s1).is_ok());
        assert!(Hash160::from_hex(s2).is_ok());
        assert!(Hash160::from_hex(s3).is_ok());

        // Invalid
        let s1 = "000000000000000000000000000000000000000"; // too short
        let s2 = "00000000000000000000000000000000000000000"; // too long
        let s3 = "000000000000000000000000000000000000000g"; // invalid character
        assert!(Hash160::from_hex(s1).is_err());
        assert!(Hash160::from_hex(s2).is_err());
        assert!(Hash160::from_hex(s3).is_err());
    }

    #[test]
    fn hash_compare() {
        let s1 = "5555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555";
        assert_eq!(
            Hash160::from_hex(s1).unwrap(),
            Hash160::from_hex(s2).unwrap()
        );

        let s1 = "0555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555";
        assert!(Hash160::from_hex(s1).unwrap() < Hash160::from_hex(s2).unwrap());

        let s1 = "5555555555555555555555555555555555555550";
        let s2 = "5555555555555555555555555555555555555555";
        assert!(Hash160::from_hex(s1).unwrap() < Hash160::from_hex(s2).unwrap());

        let s1 = "6555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555";
        assert!(Hash160::from_hex(s1).unwrap() > Hash160::from_hex(s2).unwrap());

        let s1 = "5555555555555555555555555555555555555556";
        let s2 = "5555555555555555555555555555555555555555";
        assert!(Hash160::from_hex(s1).unwrap() > Hash160::from_hex(s2).unwrap());
    }

    /// Test binary read of hash - todo
    // #[test]
    // fn hash_read() {
    //     let b = [
    //         0xbe, 0xc7, 0x7b, 0x08, 0x3c, 0xf7, 0xb7, 0x5c, 0x97, 0xcc, 0xfa, 0x0c, 0x4b, 0x0c,
    //         0x0c, 0x40, 0xa6, 0xe5, 0xae, 0x6b,
    //     ];
    //     let h = Hash160::from_binary_buf(&b[..]).unwrap();
    //     assert_eq!(
    //         h.encode_hex::<String>(),
    //         "6baee5a6400c0c4b0cfacc975cb7f73c087bc7be"
    //     );
    // }

    // #[test] todo
    // fn hash_write() {
    //     let s = "684b2f7e73dec228a7bf9a73495eeb6a28f2cda6";
    //     let h = Hash160::from_hex(s).unwrap();
    //     let b = h.to_binary_buf().unwrap();
    //     let c = vec![
    //         0xa6, 0xcd, 0xf2, 0x28, 0x6a, 0xeb, 0x5e, 0x49, 0x73, 0x9a, 0xbf, 0xa7, 0x28, 0xc2,
    //         0xde, 0x73, 0x7e, 0x2f, 0x4b, 0x68,
    //     ];
    //     assert_eq!(b, c);
    // }

    #[test]
    fn json_serialize_hash() {
        let hash = Hash160::from_hex("5643c805ff7e00fae025316393e34fa67274df4e")
            .expect("Failed to decode test hash");
        let serialized = serde_json::to_string(&hash).expect("Failed to serialize");
        // Ensure it serializes to a hex string
        assert_eq!(serialized, "\"5643c805ff7e00fae025316393e34fa67274df4e\"");
    }

    #[test]
    fn json_deserialize_hash() {
        let original_hash = Hash160::generate(b"hello world");
        let serialized = serde_json::to_string(&original_hash).expect("Failed to serialize");
        let deserialized: Hash160 =
            serde_json::from_str(&serialized).expect("Failed to deserialize");
        // Ensure the deserialized hash matches the original
        assert_eq!(deserialized, original_hash);
    }
}
