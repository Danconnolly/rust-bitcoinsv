use secp256k1::Secp256k1;
use crate::bitcoin::{base58ck, BlockchainId};
use crate::{BsvError, BsvResult};
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
    pub fn from_slice(
        data: &[u8],
    ) -> Result<PrivateKey, secp256k1::Error> {
        Ok(PrivateKey::new(secp256k1::SecretKey::from_slice(data)?))
    }

    /// Gets the WIF encoding of this private key.
    pub fn to_wif(self, kind: KeyAddressKind) -> String {
        let mut ret = Vec::with_capacity(34);
        ret[0] = kind.get_private_key_prefix();
        ret[1..33].copy_from_slice(&self.inner[..]);
        ret[33] = 1;    // always use compressed public keys
        base58ck::encode_with_checksum(&ret)
    }

    /// Parses the WIF encoded private key.
    ///
    /// Returns a tuple of the private key and the blockchain for which the private
    /// key is intended. Note that the function can not distinguish between the
    /// non-production blockchains so it will only return either BlockchainId::Main
    /// or BlockchainId::Test.
    pub fn from_wif(wif: &String) -> BsvResult<(PrivateKey, BlockchainId)> {
        let data = base58ck::decode_with_checksum(wif)?;

        let _compressed = match data.len() {
            33 => false,
            34 => true,
            _other => {
                return Err(BsvError::WifTooLong);
            }
        };

        let blockchain = match data[0] {
            0x80=> BlockchainId::Main,
            0xef => BlockchainId::Test,
            _ => {
                return Err(BsvError::InvalidBlockchainSpecifier);
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
    /// Constructs compressed ECDSA public key from the provided generic Secp256k1 public key.
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

// todo: add more tests
