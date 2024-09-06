use std::fmt::{Display, Formatter};
use crate::bitcoin::base58ck::encode_with_checksum;
use crate::bitcoin::BlockchainId;
use crate::bitcoin::crypto::{PrivateKey, PublicKey};
use crate::bitcoin::hash160::Hash160;
use crate::bitcoin::params::BlockchainParams;

/// A Bitcoin Address is a destination for a Bitcoin payment, using the P2PKH script template.
///
/// The address is the 160-bit hash of the public key, encoded in base58check format, with
/// a single byte prefix depending on the blockchain.
#[derive(Clone, Debug)]
pub struct Address {
    pub hash160: Hash160,
    pub kind: AddressKind,
}

impl Address {
    /// Get the address from a [PrivateKey] for a particular [BlockchainId].
    fn from_pv_chain(pv: &PrivateKey, blockchain: BlockchainId) -> Address {
        Address {
            hash160: Hash160::from(PublicKey::from(pv)),
            kind: AddressKind::from(blockchain),
        }
    }

    /// Get the address from a [PrivateKey] and [AddressKind].
    fn from_pv(pv: &PrivateKey, kind: AddressKind) -> Address {
        Address {
            hash160: Hash160::from(PublicKey::from(pv)),
            kind,
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut v = vec!(self.kind.get_address_prefix());
        v.append(&mut self.hash160.hash.to_vec());
        write!(f, "{}", encode_with_checksum(&v))
    }
}


/// AddressKind enables us to differentiate whether the Address is an address for the
/// production blockchain (mainnet) or whether it is for a test blockchain.
///
/// Unfortunately, the standard does not differentiate between different test blockchains.
#[derive(Clone, Debug)]
enum AddressKind {
    Main = 0,
    NotMain = 1,
}

impl AddressKind {
    pub fn get_address_prefix(&self) -> u8 {
        match self {
            AddressKind::Main => 0x00,
            AddressKind::NotMain => 0x80,
        }
    }
}

impl From<BlockchainId> for AddressKind {
    fn from(value: BlockchainId) -> Self {
        match value {
            BlockchainId::Main => AddressKind::Main,
            _ => AddressKind::NotMain,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bitcoin::crypto::{PrivateKey};
    use super::*;

    #[test]
    fn test_mainnet() {
        let (pv, n) = PrivateKey::from_wif(&"KwTeZVihYnMmcKP5MEfMeN1V726HNKFF84dWzEcqjyc7afgfyn5x".to_string()).unwrap();
        assert_eq!(n, BlockchainId::Main);
        let addr = Address::from_pv_chain(&pv, n);
        assert_eq!(addr.kind, AddressKind::Main);
        assert_eq!(addr.to_string(), "1C4UbrvcfKKTugSYRD5MKtvqTkrKMwgEHb".to_string());
    }
}
