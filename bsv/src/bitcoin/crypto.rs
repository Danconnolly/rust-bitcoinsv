use crate::bitcoin::base58ck;
use crate::bitcoin::hash160::Hash160;
use crate::bitcoin::params::KeyAddressKind;
use crate::{Error, Result};
use secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::str::FromStr;

/// A Bitcoin private key.
///
/// This is a wrapper around [secp256k1::SecretKey], providing some Bitcoin specific functionality.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateKey {
    /// The actual ECDSA key.
    pub inner: secp256k1::SecretKey,
}

impl PrivateKey {
    /// Constructs new compressed ECDSA private key using the secp256k1 algorithm and
    /// a secure random number generator.
    pub fn generate() -> PrivateKey {
        let secret_key = secp256k1::SecretKey::new(&mut rand::thread_rng());
        PrivateKey::new(secret_key)
    }

    /// Constructs private key from the provided generic Secp256k1 private key.
    pub fn new(key: secp256k1::SecretKey) -> PrivateKey {
        PrivateKey { inner: key }
    }

    /// Serializes the private key to bytes.
    pub fn to_bytes(self) -> Vec<u8> {
        self.inner[..].to_vec()
    }

    /// Deserializes a private key from a slice.
    pub fn from_slice(data: &[u8]) -> Result<PrivateKey> {
        Ok(PrivateKey::new(secp256k1::SecretKey::from_slice(data)?))
    }

    /// Gets the WIF encoding of this private key.
    pub fn to_wif(self, kind: KeyAddressKind) -> String {
        let mut ret = Vec::with_capacity(34);
        ret.push(kind.get_private_key_prefix());
        ret.extend_from_slice(&self.inner[..]);
        ret.push(1); // always use compressed public keys
        base58ck::encode_with_checksum(&ret)
    }

    /// Parses the WIF encoded private key.
    ///
    /// Returns a tuple of the private key and the blockchain for which the private
    /// key is intended. Note that the function can not distinguish between the
    /// non-production blockchains so it must return [KeyAddressKind].
    pub fn from_wif(wif: &String) -> Result<(PrivateKey, KeyAddressKind)> {
        let data = base58ck::decode_with_checksum(wif)?;

        let _compressed = match data.len() {
            33 => false,
            34 => true,
            _other => {
                return Err(Error::WifTooLong);
            }
        };

        let blockchain = match data[0] {
            0x80 => KeyAddressKind::Main,
            0xef => KeyAddressKind::NotMain,
            _ => {
                return Err(Error::InvalidBlockchainSpecifier);
            }
        };

        Ok((
            PrivateKey {
                inner: secp256k1::SecretKey::from_slice(&data[1..33])?,
            },
            blockchain,
        ))
    }
}

/// Convert a WIF in a string to a PrivateKey.
impl TryFrom<String> for PrivateKey {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Ok(PrivateKey::from_wif(&value)?.0)
    }
}

/// A Bitcoin ECDSA public key.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PublicKey {
    /// The actual ECDSA key.
    pub inner: secp256k1::PublicKey,
}

impl PublicKey {
    /// Constructs compressed ECDSA public key from anything that can be converted into a Secp256k1
    /// public key.
    ///
    /// For example, to create from a hex string, `PublicKey::new("6f67988ec4b7bf498c9164d76b52dffdc805ff8c");`
    pub fn new(key: impl Into<secp256k1::PublicKey>) -> PublicKey {
        PublicKey { inner: key.into() }
    }

    /// Returns bitcoin 160-bit hash of the public key.
    pub fn pubkey_hash(&self) -> Hash160 {
        Hash160::generate(&self.inner.serialize())
    }

    /// Serializes the public key to bytes.
    pub fn to_bytes(self) -> Vec<u8> {
        self.inner.serialize().to_vec()
    }
}

impl From<secp256k1::PublicKey> for PublicKey {
    fn from(pk: secp256k1::PublicKey) -> PublicKey {
        PublicKey::new(pk)
    }
}

impl From<&PrivateKey> for PublicKey {
    fn from(value: &PrivateKey) -> Self {
        let secp = Secp256k1::new();
        PublicKey {
            inner: secp256k1::PublicKey::from_secret_key(&secp, &value.inner),
        }
    }
}

impl FromStr for PublicKey {
    type Err = Error;

    /// Decode a public key from the hex representation as included in a script and used by
    /// OP_CHECKSIG (e.g. from a P2PKH output script).
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(PublicKey {
            inner: secp256k1::PublicKey::from_str(s)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::Address;

    /// Test decoding a public key from the hex representation within a script.
    #[test]
    fn decode_pubkeyfrom_hex() {
        // from tx d2bb697e3555cb0e4a82f0d4990d1c826eee9f648a5efc598f648bdb524093ff, input 0
        let hex = "031adba39196c65be0e61c6ddf57b397aa246729f5b639bd5bc9b5c55cf14af107";
        let r = PublicKey::from_str(hex);
        assert!(r.is_ok());
    }

    /// Test generating a random private key and printing the WIF
    #[test]
    fn test_wif() {
        let privkey = PrivateKey::generate();
        let wif = privkey.to_wif(KeyAddressKind::Main);
        assert!(!wif.is_empty());
        let (p_key2, blk_chain) = PrivateKey::from_wif(&wif).expect("Failed to parse WIF");
        assert_eq!(privkey, p_key2);
        assert_eq!(blk_chain, KeyAddressKind::Main);
    }

    /// Test some known addresses
    #[test]
    fn test_known_addresses() {
        let stn_addr = "n2ziCHyDm8wr7owJwF3smicSBAcP17L8HS";
        let stn_wif = String::from("cU5N3pE6QnRd3rZFgv1KMvUkDwMY4Vnya3bLE5JtZG3Hb549pzDN");
        let (privkey, bchain) =
            PrivateKey::from_wif(&stn_wif).expect("Failed to parse known STN WIF");
        assert_eq!(bchain, KeyAddressKind::NotMain); // stn is indistinguishable from testnet
        let addr = Address::from_pv(&privkey, bchain);
        assert_eq!(addr.to_string(), stn_addr);
    }

    /// Test bincode serialization and deserialization
    #[test]
    fn test_bincode() {
        let privkey = PrivateKey::generate();
        let config = bincode::config::legacy();
        let e = bincode::serde::encode_to_vec(&privkey, config).expect("Failed to encode privkey");
        let (d, _) =
            bincode::serde::decode_from_slice(&e, config).expect("Failed to decode privkey");
        assert_eq!(privkey, d);

        let pubkey = PublicKey::from(&privkey);
        let e = bincode::serde::encode_to_vec(&pubkey, config).expect("Failed to encode pubkey");
        let (d, _) =
            bincode::serde::decode_from_slice(&e[..], config).expect("Failed to decode pubkey");
        assert_eq!(pubkey, d);
    }
}
