#![no_main]

use libfuzzer_sys::fuzz_target;
use bitcoinsv::bitcoin::{Tx, Encodable};
use bytes::Buf;

fuzz_target!(|data: &[u8]| {
    // Try to decode a transaction from fuzzer input
    let mut buffer = data;
    if let Ok(tx) = Tx::from_binary(&mut buffer) {
        // Verify transaction limits
        assert!(tx.inputs.len() <= 1_000_000);
        assert!(tx.outputs.len() <= 1_000_000);
        
        // Test re-encoding
        use bytes::BytesMut;
        let mut encode_buffer = BytesMut::new();
        if tx.to_binary(&mut encode_buffer).is_ok() {
            // Could verify round-trip here if needed
        }
        
        // Test transaction hash calculation
        let _ = tx.hash();
        
        // Test encoded size calculation
        let _ = tx.encoded_size();
    }
});