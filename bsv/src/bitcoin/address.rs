use crate::bitcoin::base58ck::encode_with_checksum;
use crate::bitcoin::crypto::{PrivateKey, PublicKey};
use crate::bitcoin::hash160::Hash160;
use crate::bitcoin::params::KeyAddressKind;
use crate::bitcoin::BlockchainId;
use std::fmt::{Display, Formatter};

/// A Bitcoin Address is a destination for a Bitcoin payment, using the P2PKH script template.
///
/// The address is the 160-bit hash of the public key, encoded in base58check format, with
/// a single byte prefix depending on the blockchain.
#[derive(Clone, Debug)]
pub struct Address {
    pub hash160: Hash160,
    pub kind: KeyAddressKind,
}

impl Address {
    /// Get the address from a [PrivateKey] for a particular [BlockchainId].
    pub fn from_pv_chain(pv: &PrivateKey, blockchain: BlockchainId) -> Address {
        Address {
            hash160: Hash160::from(PublicKey::from(pv)),
            kind: KeyAddressKind::from(blockchain),
        }
    }

    /// Get the address from a [PrivateKey] and [KeyAddressKind].
    pub fn from_pv(pv: &PrivateKey, kind: KeyAddressKind) -> Address {
        Address {
            hash160: Hash160::from(PublicKey::from(pv)),
            kind,
        }
    }

    /// Get the address from a [PublicKey] and [KeyAddressKind].
    pub fn from_pubkey(pubkey: &PublicKey, kind: KeyAddressKind) -> Address {
        Address {
            hash160: pubkey.pubkey_hash(),
            kind,
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut v = vec![self.kind.get_address_prefix()];
        v.append(&mut self.hash160.hash.to_vec());
        write!(f, "{}", encode_with_checksum(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::crypto::PrivateKey;
    use crate::bitcoin::params::KeyAddressKind;
    use std::str::FromStr;

    #[test]
    fn test_mainnet() {
        let (pv, n) = PrivateKey::from_wif(
            &"KwTeZVihYnMmcKP5MEfMeN1V726HNKFF84dWzEcqjyc7afgfyn5x".to_string(),
        )
        .expect("Failed to parse WIF private key for test");
        assert_eq!(n, KeyAddressKind::Main);
        let addr = Address::from_pv(&pv, n);
        assert_eq!(addr.kind, KeyAddressKind::Main);
        assert_eq!(
            addr.to_string(),
            "1C4UbrvcfKKTugSYRD5MKtvqTkrKMwgEHb".to_string()
        );
    }

    /// Create a public key from a hex literal extracted from a confirmed mainnet tx, then
    /// check the address representation of the key.
    #[test]
    fn create_pubkey_from_hex() {
        // from tx d2bb697e3555cb0e4a82f0d4990d1c826eee9f648a5efc598f648bdb524093ff input 0
        let key = PublicKey::from_str(
            "031adba39196c65be0e61c6ddf57b397aa246729f5b639bd5bc9b5c55cf14af107",
        )
        .expect("Failed to parse public key from hex for test");
        let addr = Address::from_pubkey(&key, KeyAddressKind::Main);
        assert_eq!(
            addr.to_string(),
            "1BA47GLhQZrTtPt21CJ73cY9YSSsCXX7gF".to_string()
        );
    }

    /// Create a public key from a hex literal extracted from a confirmed STN tx, then
    /// check the address representation of the key.
    #[test]
    fn stn_create_pubkey_from_hex() {
        // from tx 7e2b52e6d6b803038bfbbe662ae02ae2c27e4cf27ba25a9be7bdeff3fab59eea input 0
        let key = PublicKey::from_str(
            "0319a115644e45738b06b7274b45c36c7c1cbecef32d667b9246eba684939ca1bc",
        )
        .expect("Failed to parse public key from hex for STN test");
        let addr = Address::from_pubkey(&key, KeyAddressKind::NotMain);
        assert_eq!(
            addr.to_string(),
            "mvkjTSVMgGnSmjxkrDmYL6qKv9f5Hzefog".to_string()
        );
    }
}
