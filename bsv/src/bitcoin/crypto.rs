use std::str::FromStr;
use secp256k1::Secp256k1;
use crate::bitcoin::{base58ck, BlockchainId};
use crate::{Error, Result};
use crate::bitcoin::hash160::Hash160;
use crate::bitcoin::params::KeyAddressKind;


/// A Bitcoin private key.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    pub fn to_bytes(self) -> Vec<u8> { self.inner[..].to_vec() }

    /// Deserializes a private key from a slice.
    ///
    /// todo: why is this returning a std::result::Result?
    pub fn from_slice(
        data: &[u8],
    ) -> std::result::Result<PrivateKey, secp256k1::Error> {
        Ok(PrivateKey::new(secp256k1::SecretKey::from_slice(data)?))
    }

    /// Gets the WIF encoding of this private key.
    pub fn to_wif(self, kind: KeyAddressKind) -> String {
        let mut ret = Vec::with_capacity(34);
        ret.push(kind.get_private_key_prefix());
        ret.extend_from_slice(&self.inner[..]);
        ret.push(1);    // always use compressed public keys
        base58ck::encode_with_checksum(&ret)
    }

    /// Parses the WIF encoded private key.
    ///
    /// Returns a tuple of the private key and the blockchain for which the private
    /// key is intended. Note that the function can not distinguish between the
    /// non-production blockchains so it will only return either BlockchainId::Main
    /// or BlockchainId::Test.
    pub fn from_wif(wif: &String) -> Result<(PrivateKey, BlockchainId)> {
        let data = base58ck::decode_with_checksum(wif)?;

        let _compressed = match data.len() {
            33 => false,
            34 => true,
            _other => {
                return Err(Error::WifTooLong);
            }
        };

        let blockchain = match data[0] {
            0x80=> BlockchainId::Main,
            0xef => BlockchainId::Test,
            _ => {
                return Err(Error::InvalidBlockchainSpecifier);
            }
        };

        Ok((PrivateKey {
            inner: secp256k1::SecretKey::from_slice(&data[1..33])?,
        }, blockchain))
    }
}

impl From<String> for PrivateKey {
    fn from(value: String) -> Self {
        PrivateKey::from_wif(&value).unwrap().0
    }
}


/// A Bitcoin ECDSA public key.
#[derive(Debug, Copy, Clone)]
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
    fn from(pk: secp256k1::PublicKey) -> PublicKey { PublicKey::new(pk) }
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
            inner: secp256k1::PublicKey::from_str(s)?
        })
    }
}


// todo: add more tests
#[cfg(test)]
mod tests {
    use hex_literal::len;
    use super::*;

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
        assert!(wif.len() > 0);
        let (p_key2, blk_chain) = PrivateKey::from_wif(&wif).unwrap();
        assert_eq!(privkey, p_key2);
        assert_eq!(blk_chain, BlockchainId::Main);
    }
}