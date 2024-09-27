use bytes::Bytes;
use num::{BigInt, ToPrimitive};
use crate::{Error, Result};

/// A data value that is used in Bitcoin Script.
///
/// The values in Bitcoin Script are sequences of bytes. These sequences may be interpreted as
/// boolean or numeric values by some operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteSequence {
    raw: Bytes,
}

impl ByteSequence {
    /// Create a new byte sequence.
    pub fn new(data: Bytes) -> Self {
        ByteSequence {
            raw: data
        }
    }

    /// Get the bytes from the byte sequence.
    pub fn get_bytes(&self) -> Bytes {
        self.raw.clone()
    }

    /// Get the length of the byte sequence.
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Can the byte sequence represent a small number (i64)?
    pub fn is_small_num(&self) -> bool {
        self.len() <= 8
    }

    /// Return the value as a small number (i64).
    pub fn to_small_number(&self) -> Result<i64> {
        if self.raw.len() > 8 {
            Err(Error::DataTooLarge)
        } else if self.raw.len() == 0 {
            Ok(0)
        } else {
            // Using bigint's so we can handle numerics with strange sizes such as 3 bytes
            let i = BigInt::from_signed_bytes_le(&self.raw[..]);
            match i.to_i64() {
                None => Err(Error::DataTooLarge),
                Some(val) => Ok(val),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Some tests that it evaluates small numbers correctly
    #[test]
    fn check_small_numbers() {
        // null byte sequence
        let i = ByteSequence::new(Bytes::from(vec![]));
        assert_eq!(i.len(), 0);
        assert_eq!(i.is_small_num(), true);
        assert_eq!(i.to_small_number().unwrap(), 0);

        // zero
        let i = ByteSequence::new(Bytes::from(vec![0u8]));
        assert_eq!(i.len(), 1);
        assert_eq!(i.is_small_num(), true);
        assert_eq!(i.to_small_number().unwrap(), 0);

        // random single byte value
        let i = ByteSequence::new(Bytes::from(vec![23u8]));
        assert_eq!(i.len(), 1);
        assert_eq!(i.is_small_num(), true);
        assert_eq!(i.to_small_number().unwrap(), 23);

        // 2 byte value
        let i = ByteSequence::new(Bytes::from(vec![1u8, 14u8]));
        assert_eq!(i.len(), 2);
        assert_eq!(i.is_small_num(), true);
        assert_eq!(i.to_small_number().unwrap(), 256*14+1);

        // -1
        let i = ByteSequence::new(Bytes::from(vec![255u8]));
        assert_eq!(i.len(), 1);
        assert_eq!(i.is_small_num(), true);
        assert_eq!(i.to_small_number().unwrap(), -1);

        // too large
        let i = ByteSequence::new(Bytes::from(vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9]));
        assert_eq!(i.len(), 9);
        assert_eq!(i.is_small_num(), false);
        assert!(i.to_small_number().is_err());

        // too large but has leading zero so it could actually resolve but is against rules
        let i = ByteSequence::new(Bytes::from(vec![0u8, 2, 3, 4, 5, 6, 7, 8, 9]));
        assert_eq!(i.len(), 9);
        assert_eq!(i.is_small_num(), false);
        assert!(i.to_small_number().is_err());

        // 3 byte value
        let i = ByteSequence::new(Bytes::from(vec![1u8, 2, 3]));
        assert_eq!(i.len(), 3);
        assert_eq!(i.is_small_num(), true);
        assert_eq!(i.to_small_number().unwrap(), ((3 * 256) + 2) * 256 + 1);

        // 8 byte value, no leading zero
        let i = ByteSequence::new(Bytes::from(vec![1u8, 2, 3, 4, 5, 6, 7, 8]));
        assert_eq!(i.len(), 8);
        assert_eq!(i.is_small_num(), true);
        assert_eq!(i.to_small_number().unwrap(), (((((((8 * 256 + 7) * 256 + 6) * 256 + 5) * 256 + 4) * 256 + 3) * 256) + 2) * 256 + 1);
    }
}