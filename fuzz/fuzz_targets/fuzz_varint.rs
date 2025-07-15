#![no_main]

use libfuzzer_sys::fuzz_target;
use bitcoinsv::bitcoin::{varint_encode, varint_decode, varint_size};
use bytes::{BytesMut, Buf};

fuzz_target!(|data: &[u8]| {
    // Test varint decoding
    if !data.is_empty() {
        let mut buffer = data;
        if let Ok(value) = varint_decode(&mut buffer) {
            // If we successfully decoded a value, verify round-trip
            let mut encode_buffer = BytesMut::new();
            if varint_encode(&mut encode_buffer, value).is_ok() {
                // Verify size calculation
                let calculated_size = varint_size(value);
                assert_eq!(calculated_size as usize, encode_buffer.len());
                
                // Verify round-trip
                let mut decode_buffer = encode_buffer.as_ref();
                if let Ok(decoded) = varint_decode(&mut decode_buffer) {
                    assert_eq!(value, decoded);
                }
            }
        }
    }
});