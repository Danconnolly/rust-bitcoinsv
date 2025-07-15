#![no_main]

use libfuzzer_sys::fuzz_target;
use bitcoinsv::bitcoin::{Hash, calculate_merkle_root, build_merkle_proof, verify_merkle_proof};

fuzz_target!(|data: &[u8]| {
    // Create transaction hashes from fuzzer input
    // Ensure we have at least 32 bytes for one hash
    if data.len() >= 32 {
        let num_hashes = (data.len() / 32).min(1000); // Limit to prevent OOM
        let mut tx_hashes = Vec::new();
        
        for i in 0..num_hashes {
            let start = i * 32;
            let end = start + 32;
            if end <= data.len() {
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(&data[start..end]);
                tx_hashes.push(Hash { raw: hash_bytes });
            }
        }
        
        if !tx_hashes.is_empty() {
            // Test merkle root calculation
            if let Ok(root) = calculate_merkle_root(&tx_hashes) {
                // Test merkle proof generation and verification
                for (index, tx_hash) in tx_hashes.iter().enumerate() {
                    if let Ok(proof) = build_merkle_proof(&tx_hashes, index) {
                        // Verify the proof
                        let is_valid = verify_merkle_proof(tx_hash, index, &proof, &root);
                        assert!(is_valid, "Valid proof should verify");
                    }
                }
            }
        }
    }
});