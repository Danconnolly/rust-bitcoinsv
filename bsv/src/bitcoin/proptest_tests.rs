//! Property-based tests using proptest
//!
//! These tests use proptest to generate random inputs and verify
//! properties that should always hold true.

#[cfg(test)]
mod tests {
    use crate::bitcoin::*;
    use bytes::{Buf, BytesMut};
    use proptest::prelude::*;

    // Strategy for generating valid Bitcoin amounts (0 to 21 million BTC in satoshis)
    fn bitcoin_amount() -> impl Strategy<Value = u64> {
        0u64..=21_000_000_00000000u64
    }

    // Strategy for generating transaction counts
    fn tx_count() -> impl Strategy<Value = usize> {
        1usize..=1000usize
    }

    // Strategy for generating byte arrays of specific size
    fn byte_array_32() -> impl Strategy<Value = [u8; 32]> {
        any::<[u8; 32]>()
    }

    proptest! {
        #[test]
        fn test_varint_roundtrip(value: u64) {
            // Property: encoding and decoding a varint should give back the original value
            let mut encode_buffer = BytesMut::new();
            varint_encode(&mut encode_buffer, value).unwrap();

            let mut decode_buffer = encode_buffer.as_ref();
            let decoded = varint_decode(&mut decode_buffer).unwrap();

            prop_assert_eq!(value, decoded);
            prop_assert_eq!(decode_buffer.remaining(), 0, "Buffer should be fully consumed");
        }

        #[test]
        fn test_varint_size_consistency(value: u64) {
            // Property: varint_size should match actual encoded size
            let calculated_size = varint_size(value) as usize;

            let mut buffer = BytesMut::new();
            varint_encode(&mut buffer, value).unwrap();
            let actual_size = buffer.len();

            prop_assert_eq!(calculated_size, actual_size);
        }

        #[test]
        fn test_hash_deterministic(data: Vec<u8>) {
            // Property: hashing the same data should always produce the same result
            let hash1 = Hash::sha256d(&data);
            let hash2 = Hash::sha256d(&data);

            prop_assert_eq!(hash1, hash2);
        }

        #[test]
        fn test_hash_hex_roundtrip(bytes in byte_array_32()) {
            // Property: converting hash to hex and back should preserve the value
            let hash = Hash { raw: bytes };
            // Use the Hash's ToHex implementation which reverses bytes
            let hex_string = hash.encode_hex::<String>();
            let decoded = Hash::from_hex(&hex_string).unwrap();

            prop_assert_eq!(hash, decoded);
        }

        #[test]
        fn test_merkle_root_properties(
            tx_count in tx_count(),
            seed: u64
        ) {
            // Generate deterministic transaction hashes
            use rand::{SeedableRng, RngCore};
            use rand::rngs::StdRng;

            let mut rng = StdRng::seed_from_u64(seed);
            let mut tx_hashes = Vec::new();

            for _ in 0..tx_count {
                let mut hash_bytes = [0u8; 32];
                rng.fill_bytes(&mut hash_bytes);
                tx_hashes.push(Hash { raw: hash_bytes });
            }

            // Property 1: Merkle root should be deterministic
            let root1 = calculate_merkle_root(&tx_hashes).unwrap();
            let root2 = calculate_merkle_root(&tx_hashes).unwrap();
            prop_assert_eq!(root1, root2);

            // Property 2: All proofs should verify
            for (index, tx_hash) in tx_hashes.iter().enumerate() {
                let proof = build_merkle_proof(&tx_hashes, index).unwrap();
                let is_valid = verify_merkle_proof(tx_hash, index, &proof, &root1);
                prop_assert!(is_valid, "Proof should be valid for index {}", index);
            }
        }

        #[test]
        fn test_script_builder_operations(
            push_data: Vec<u8>,
            num_value in 0i64..=16i64
        ) {
            // Property: building a script and parsing it should preserve operations
            use crate::bitcoin::script::{ScriptBuilder, Operation, ByteSequence};

            let mut builder = ScriptBuilder::new();

            // Add some operations
            if !push_data.is_empty() && push_data.len() <= 520 {
                let bytes = ByteSequence::new(bytes::Bytes::from(push_data.clone()));
                builder.add(Operation::OP_PUSH(bytes));
            }

            // Add a number operation based on value
            if num_value >= 1 && num_value <= 16 {
                use crate::bitcoin::script::Operation::*;
                let op = match num_value {
                    1 => OP_1,
                    2 => OP_2,
                    3 => OP_3,
                    4 => OP_4,
                    5 => OP_5,
                    6 => OP_6,
                    7 => OP_7,
                    8 => OP_8,
                    9 => OP_9,
                    10 => OP_10,
                    11 => OP_11,
                    12 => OP_12,
                    13 => OP_13,
                    14 => OP_14,
                    15 => OP_15,
                    16 => OP_16,
                    _ => unreachable!(),
                };
                builder.add(op);
            }

            // Add some standard operations
            builder.add(Operation::OP_DUP);
            builder.add(Operation::OP_HASH160);

            let script = builder.build().unwrap();

            // Verify we can parse the operations back
            let ops = script.operations().unwrap();
            prop_assert!(ops.len() >= 2, "Should have at least the standard operations");
        }

        #[test]
        fn test_network_address_roundtrip(
            ip_bytes: [u8; 4],
            port: u16,
            services: u64
        ) {
            // Property: encoding and decoding a network address should preserve values
            use crate::p2p::{NetworkAddress, Services};
            use std::net::{IpAddr, Ipv4Addr};

            let addr = NetworkAddress {
                timestamp: None,
                services: Services(services),
                addr: IpAddr::V4(Ipv4Addr::new(ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3])),
                port,
            };

            let mut buffer = BytesMut::new();
            addr.encode(&mut buffer);

            let mut decode_buffer = buffer.freeze();
            let decoded = NetworkAddress::decode(&mut decode_buffer).unwrap();

            prop_assert_eq!(addr.services, decoded.services);
            prop_assert_eq!(addr.addr, decoded.addr);
            prop_assert_eq!(addr.port, decoded.port);
        }

        #[test]
        fn test_amount_satoshi_conversion(satoshis in bitcoin_amount()) {
            // Property: converting between satoshis and Amount should be lossless
            use crate::util::Amount;

            let amount = Amount::from_satoshis(satoshis as i64);
            let recovered = amount.satoshis as u64;

            prop_assert_eq!(satoshis, recovered);
        }

        #[test]
        fn test_private_key_properties(seed: [u8; 32]) {
            // Property: private key operations should be consistent
            use secp256k1::SecretKey;

            // Skip if the seed doesn't create a valid private key
            if let Ok(secret_key) = SecretKey::from_slice(&seed) {
                let privkey = PrivateKey::new(secret_key);
                let pubkey1 = PublicKey::from(&privkey);
                let pubkey2 = PublicKey::from(&privkey);

                // Same private key should always produce same public key
                prop_assert_eq!(pubkey1, pubkey2);

                // Public key should have valid length
                let pubkey_bytes = pubkey1.to_bytes();
                prop_assert!(pubkey_bytes.len() == 33 || pubkey_bytes.len() == 65);
            }
        }

        #[test]
        fn test_script_number_conversion(
            value in i64::MIN/2..=i64::MAX/2  // Avoid overflow in operations
        ) {
            // Property: converting to and from script numbers should preserve value
            use crate::bitcoin::script::ByteSequence;
            use num::{BigInt, ToPrimitive};

            // Convert to bytes using BigInt
            let bigint = BigInt::from(value);
            let bytes = bigint.to_signed_bytes_le();

            // Convert back
            let recovered_bigint = BigInt::from_signed_bytes_le(&bytes);
            let recovered = recovered_bigint.to_i64().unwrap();

            prop_assert_eq!(value, recovered);

            // Also test through ByteSequence if within size limit
            if bytes.len() <= 8 {
                let byte_seq = ByteSequence::new(bytes::Bytes::from(bytes));
                if let Ok(num) = byte_seq.to_small_number() {
                    prop_assert_eq!(value, num);
                }
            }
        }
    }

    proptest! {
        // Stress test properties with larger inputs

        #[test]
        fn test_large_merkle_tree(
            tx_count in 100usize..=10000usize,
            seed: u64
        ) {
            // Property: large merkle trees should still maintain all properties
            use rand::{SeedableRng, RngCore};
            use rand::rngs::StdRng;

            let mut rng = StdRng::seed_from_u64(seed);
            let mut tx_hashes = Vec::new();

            for _ in 0..tx_count {
                let mut hash_bytes = [0u8; 32];
                rng.fill_bytes(&mut hash_bytes);
                tx_hashes.push(Hash { raw: hash_bytes });
            }

            // Should be able to calculate root without panic
            let root = calculate_merkle_root(&tx_hashes).unwrap();

            // Spot check: first and last transaction proofs should verify
            let first_proof = build_merkle_proof(&tx_hashes, 0).unwrap();
            prop_assert!(verify_merkle_proof(&tx_hashes[0], 0, &first_proof, &root));

            let last_idx = tx_hashes.len() - 1;
            let last_proof = build_merkle_proof(&tx_hashes, last_idx).unwrap();
            prop_assert!(verify_merkle_proof(&tx_hashes[last_idx], last_idx, &last_proof, &root));
        }

        #[test]
        fn test_varint_encoding_distribution(
            values in prop::collection::vec(any::<u64>(), 1..1000)
        ) {
            // Property: encoding multiple varints should be decodable in sequence
            let mut buffer = BytesMut::new();

            // Encode all values
            for value in &values {
                varint_encode(&mut buffer, *value).unwrap();
            }

            // Decode all values
            let mut decoded_values = Vec::new();
            while buffer.has_remaining() {
                decoded_values.push(varint_decode(&mut buffer).unwrap());
            }

            prop_assert_eq!(values, decoded_values);
        }
    }
}
