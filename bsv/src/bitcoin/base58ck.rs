use crate::bitcoin::Hash;
use crate::{Error, Result};
use base58::{FromBase58, ToBase58};

/// Functions for base-58 encoding with checksum.
///
/// Some Bitcoin standards use base-58 encoding with an additional checksum. The checksum
/// is appended to the end of the base-58 encoding and consists of the first 4 bytes of
/// the SHA256D hash of the value.

/// Encodes `data` as a base58 string including the checksum.
///
/// The checksum is the first four bytes of the sha256d of the data and is concatenated onto the end.
pub fn encode_with_checksum(data: &Vec<u8>) -> String {
    let mut checksum = Hash::sha256d(data).raw[0..4].to_vec();
    let mut ck_data = data.clone();
    ck_data.append(&mut checksum);
    ck_data.to_base58()
}

/// Decode from base58 with checksum, verifying and removing the checksum.
///
/// A Box<> is returned instead of a Vec<> because it performs slightly better and we do not expect
/// modification of the returned data will be needed.
///
/// The checksum is the first four bytes of the sha256d of the data and is concatenated onto the end
/// of the base58 encoding.
pub fn decode_with_checksum(encoded: &String) -> Result<Vec<u8>> {
    let mut data = encoded.from_base58()?;
    let l = data.len();
    if l < 5 {
        Err(Error::BadData(
            "base58 string too short to contain checksum".to_string(),
        ))
    } else {
        let ck = Hash::sha256d(&data[..l - 4]);
        if ck.raw[0..4] != data[l - 4..] {
            Err(Error::ChecksumMismatch)
        } else {
            data.truncate(l - 4);
            Ok(data)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bitcoin::base58ck::{decode_with_checksum, encode_with_checksum};
    use hex_literal::hex;

    #[test]
    fn test_base58ck_encode() {
        // from output 0 of tx 1e155211334dfcf345cf257fabbf8fcc5f665f26cd5d612f1b5331ff3ec950fa
        // 160 hash is 2c7a568d346629f5308a5b75d825d28b09297153
        // prepend 0x00 for mainnet address
        let addr = hex!("002c7a568d346629f5308a5b75d825d28b09297153");
        assert_eq!(
            encode_with_checksum(&Vec::from(addr)),
            "154BHe8d7Dmm7pWLG8J9gceXiCfCRDtWAo"
        );
    }

    #[test]
    fn test_base58ck_decode() {
        let h = hex!("002c7a568d346629f5308a5b75d825d28b09297153");
        let addr: String = "154BHe8d7Dmm7pWLG8J9gceXiCfCRDtWAo".to_string();
        let r = decode_with_checksum(&addr).expect("Failed to decode test address");
        assert_eq!(*r, h);
    }
}
