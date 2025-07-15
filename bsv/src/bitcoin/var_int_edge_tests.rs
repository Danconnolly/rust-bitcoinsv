//! Edge case tests for VarInt encoding/decoding
//!
//! This module contains comprehensive tests for edge cases in VarInt
//! encoding and decoding, including boundary values, error conditions,
//! and buffer handling.

#[cfg(test)]
mod tests {
    use crate::bitcoin::var_int::*;
    use bytes::{Buf, BufMut, BytesMut};
    use std::io::Cursor;

    #[test]
    fn test_boundary_values() {
        // Test all boundary values for size transitions
        let boundaries = vec![
            (0u64, 1usize, vec![0x00]),
            (252u64, 1usize, vec![0xFC]),
            (253u64, 3usize, vec![0xFD, 0xFD, 0x00]),
            (0xFFFFu64, 3usize, vec![0xFD, 0xFF, 0xFF]),
            (0x10000u64, 5usize, vec![0xFE, 0x00, 0x00, 0x01, 0x00]),
            (0xFFFFFFFFu64, 5usize, vec![0xFE, 0xFF, 0xFF, 0xFF, 0xFF]),
            (
                0x100000000u64,
                9usize,
                vec![0xFF, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00],
            ),
            (
                u64::MAX,
                9usize,
                vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            ),
        ];

        for (value, expected_size, expected_bytes) in boundaries {
            // Test size calculation
            assert_eq!(
                varint_size(value) as usize,
                expected_size,
                "Size mismatch for value {}",
                value
            );

            // Test encoding
            let mut buf = BytesMut::new();
            varint_encode(&mut buf, value).unwrap();
            assert_eq!(
                buf.to_vec(),
                expected_bytes,
                "Encoding mismatch for value {}",
                value
            );

            // Test decoding
            let mut cursor = Cursor::new(&expected_bytes);
            let decoded = varint_decode(&mut cursor).unwrap();
            assert_eq!(decoded, value, "Decoding mismatch for value {}", value);
        }
    }

    #[test]
    fn test_buffer_underflow() {
        // Test decoding with insufficient buffer data
        let test_cases = vec![
            vec![0xFD],                                           // Needs 3 bytes, only has 1
            vec![0xFD, 0x00],                                     // Needs 3 bytes, only has 2
            vec![0xFE],                                           // Needs 5 bytes, only has 1
            vec![0xFE, 0x00, 0x00],                               // Needs 5 bytes, only has 3
            vec![0xFE, 0x00, 0x00, 0x00],                         // Needs 5 bytes, only has 4
            vec![0xFF],                                           // Needs 9 bytes, only has 1
            vec![0xFF, 0x00, 0x00, 0x00, 0x00],                   // Needs 9 bytes, only has 5
            vec![0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // Needs 9 bytes, only has 8
        ];

        for data in test_cases {
            let mut cursor = Cursor::new(&data);
            let result = std::panic::catch_unwind(move || {
                let _ = varint_decode(&mut cursor);
            });
            assert!(
                result.is_err(),
                "Expected panic for insufficient data: {:?}",
                data
            );
        }
    }

    #[test]
    fn test_off_by_one_values() {
        // Test values around boundaries to ensure correct size selection
        let test_values = vec![
            // Around 252/253 boundary
            (251u64, 1usize),
            (252u64, 1usize),
            (253u64, 3usize),
            (254u64, 3usize),
            // Around 0xFFFF/0x10000 boundary
            (0xFFFEu64, 3usize),
            (0xFFFFu64, 3usize),
            (0x10000u64, 5usize),
            (0x10001u64, 5usize),
            // Around 0xFFFFFFFF/0x100000000 boundary
            (0xFFFFFFFEu64, 5usize),
            (0xFFFFFFFFu64, 5usize),
            (0x100000000u64, 9usize),
            (0x100000001u64, 9usize),
        ];

        for (value, expected_size) in test_values {
            assert_eq!(
                varint_size(value) as usize,
                expected_size,
                "Size mismatch for value {}",
                value
            );

            // Encode and decode to verify correctness
            let mut buf = BytesMut::new();
            varint_encode(&mut buf, value).unwrap();
            assert_eq!(
                buf.len(),
                expected_size,
                "Encoded size mismatch for value {}",
                value
            );

            let mut buf_cursor = buf.clone();
            let decoded = varint_decode(&mut buf_cursor).unwrap();
            assert_eq!(decoded, value, "Round-trip mismatch for value {}", value);
        }
    }

    #[test]
    fn test_multiple_varints_in_buffer() {
        // Test encoding and decoding multiple varints in sequence
        let values = vec![
            0u64,
            252,
            253,
            0xFFFF,
            0x10000,
            0xFFFFFFFF,
            0x100000000,
            u64::MAX,
        ];

        let mut buf = BytesMut::new();
        for value in &values {
            varint_encode(&mut buf, *value).unwrap();
        }

        // Decode all values
        let mut decoded_values = Vec::new();
        while buf.has_remaining() {
            decoded_values.push(varint_decode(&mut buf).unwrap());
        }

        assert_eq!(
            values, decoded_values,
            "Multiple varint encoding/decoding failed"
        );
    }

    #[test]
    fn test_size_calculation_consistency() {
        // Ensure size calculation matches actual encoded size
        let test_values = vec![
            0,
            1,
            127,
            252,
            253,
            254,
            1000,
            0xFFFF,
            0x10000,
            0x10001,
            1000000,
            0xFFFFFFFF,
            0x100000000,
            u64::MAX / 2,
            u64::MAX - 1,
            u64::MAX,
        ];

        for value in test_values {
            let calculated_size = varint_size(value) as usize;

            let mut buf = BytesMut::new();
            varint_encode(&mut buf, value).unwrap();
            let actual_size = buf.len();

            assert_eq!(
                calculated_size, actual_size,
                "Size calculation mismatch for value {}. Calculated: {}, Actual: {}",
                value, calculated_size, actual_size
            );
        }
    }

    #[test]
    fn test_special_bitcoin_values() {
        // Test values commonly seen in Bitcoin protocol
        let bitcoin_values = vec![
            (0u64, "Empty list/array"),
            (1u64, "Single item"),
            (50u64, "Typical transaction input/output count"),
            (520u64, "Maximum standard script element size"),
            (10000u64, "Maximum standard script size"),
            (100000u64, "Typical block transaction count"),
            (1000000u64, "Large block transaction count"),
            (21000000u64, "Bitcoin max supply in BTC"),
            (2100000000000000u64, "Bitcoin max supply in satoshis"),
        ];

        for (value, description) in bitcoin_values {
            let mut buf = BytesMut::new();
            varint_encode(&mut buf, value).unwrap();

            let mut buf_cursor = buf.clone();
            let decoded = varint_decode(&mut buf_cursor).unwrap();

            assert_eq!(
                decoded, value,
                "Bitcoin value test failed for {} ({})",
                value, description
            );
        }
    }

    #[test]
    fn test_buffer_capacity() {
        // Test that encoding respects buffer capacity
        let mut buf = BytesMut::with_capacity(1);

        // This should succeed and grow the buffer
        varint_encode(&mut buf, 0xFFFFFFFF).unwrap();
        assert_eq!(buf.len(), 5);

        // Test with exact capacity
        let mut buf2 = BytesMut::with_capacity(9);
        varint_encode(&mut buf2, u64::MAX).unwrap();
        assert_eq!(buf2.len(), 9);
    }

    #[test]
    fn test_endianness() {
        // Verify little-endian encoding for multi-byte values
        let test_cases = vec![
            (0x0102u64, vec![0xFD, 0x02, 0x01]),
            (0x01020304u64, vec![0xFE, 0x04, 0x03, 0x02, 0x01]),
            (
                0x0102030405060708u64,
                vec![0xFF, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01],
            ),
        ];

        for (value, expected_bytes) in test_cases {
            let mut buf = BytesMut::new();
            varint_encode(&mut buf, value).unwrap();
            assert_eq!(
                buf.to_vec(),
                expected_bytes,
                "Endianness test failed for value 0x{:X}",
                value
            );
        }
    }

    #[test]
    fn test_zero_remaining_buffer() {
        // Test behavior with zero remaining buffer space
        let data = vec![0x00];
        let mut cursor = Cursor::new(&data);

        // Read the first byte
        let value = varint_decode(&mut cursor).unwrap();
        assert_eq!(value, 0);

        // Now buffer has zero remaining
        assert!(!cursor.has_remaining());

        // Attempting to read should panic
        let result = std::panic::catch_unwind(move || {
            let _ = varint_decode(&mut cursor);
        });
        assert!(
            result.is_err(),
            "Expected panic when reading from empty buffer"
        );
    }

    #[test]
    fn test_random_round_trips() {
        // Test random values to ensure encode/decode consistency
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let mut rng = StdRng::seed_from_u64(42); // Deterministic for reproducibility

        for _ in 0..1000 {
            let value = rng.gen::<u64>();

            let mut buf = BytesMut::new();
            varint_encode(&mut buf, value).unwrap();

            let mut buf_cursor = buf.clone();
            let decoded = varint_decode(&mut buf_cursor).unwrap();

            assert_eq!(
                decoded, value,
                "Random round-trip test failed for value {}",
                value
            );

            // Verify no extra bytes were consumed
            assert_eq!(
                buf_cursor.remaining(),
                0,
                "Buffer not fully consumed for value {}",
                value
            );
        }
    }

    #[test]
    fn test_chained_buffer_operations() {
        // Test varint operations with chained buffer operations
        let mut buf = BytesMut::new();

        // Chain multiple operations
        buf.put_u8(0x42); // Random byte
        varint_encode(&mut buf, 1000).unwrap();
        buf.put_u16_le(0x1234); // Random u16
        varint_encode(&mut buf, 0xFFFFFFFF).unwrap();
        buf.put_u32_le(0x56789ABC); // Random u32
        varint_encode(&mut buf, u64::MAX).unwrap();

        // Now decode, skipping the non-varint data
        let mut cursor = buf.clone();
        assert_eq!(cursor.get_u8(), 0x42);
        assert_eq!(varint_decode(&mut cursor).unwrap(), 1000);
        assert_eq!(cursor.get_u16_le(), 0x1234);
        assert_eq!(varint_decode(&mut cursor).unwrap(), 0xFFFFFFFF);
        assert_eq!(cursor.get_u32_le(), 0x56789ABC);
        assert_eq!(varint_decode(&mut cursor).unwrap(), u64::MAX);
        assert_eq!(cursor.remaining(), 0);
    }
}
