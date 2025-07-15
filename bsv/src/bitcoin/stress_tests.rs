//! Stress tests for large data handling
//!
//! These tests verify that the library can handle large amounts of data
//! without panicking or running out of memory.

#[cfg(test)]
mod tests {
    use crate::bitcoin::*;
    use crate::util::Amount;
    use bytes::{Buf, BufMut, BytesMut};
    use std::time::Instant;

    #[test]
    #[ignore] // Run with: cargo test stress_tests:: -- --ignored
    fn test_large_transaction_with_many_inputs() {
        // Test handling a transaction with many inputs (near the limit)
        const NUM_INPUTS: usize = 10_000; // Well below the 1M limit but still large

        println!("Creating transaction with {} inputs...", NUM_INPUTS);
        let start = Instant::now();

        // Create a transaction with many inputs
        let mut inputs = Vec::new();
        for i in 0..NUM_INPUTS {
            let mut hash_bytes = [0u8; 32];
            hash_bytes[0..4].copy_from_slice(&(i as u32).to_le_bytes());

            let outpoint = Outpoint {
                raw: {
                    let mut buf = BytesMut::with_capacity(36);
                    buf.put_slice(&hash_bytes);
                    buf.put_u32_le(0);
                    buf.freeze()
                },
            };

            inputs.push(TxInput {
                outpoint,
                script: Script {
                    raw: bytes::Bytes::new(),
                },
                sequence: 0xFFFFFFFF,
            });
        }

        // Create a simple output
        let outputs = vec![TxOutput {
            value: 50_000,
            script: Script {
                raw: bytes::Bytes::from_static(b""),
            },
        }];

        let tx = Tx {
            version: 1,
            inputs,
            outputs,
            lock_time: 0,
        };

        let creation_time = start.elapsed();
        println!("Transaction created in {:?}", creation_time);

        // Test encoding
        let encode_start = Instant::now();
        let mut buffer = BytesMut::new();
        tx.to_binary(&mut buffer).unwrap();
        let encode_time = encode_start.elapsed();
        println!(
            "Transaction encoded in {:?}, size: {} bytes",
            encode_time,
            buffer.len()
        );

        // Test decoding
        let decode_start = Instant::now();
        let mut decode_buffer = buffer.freeze();
        let decoded_tx = Tx::from_binary(&mut decode_buffer).unwrap();
        let decode_time = decode_start.elapsed();
        println!("Transaction decoded in {:?}", decode_time);

        assert_eq!(decoded_tx.inputs.len(), NUM_INPUTS);
        assert_eq!(decoded_tx.outputs.len(), 1);
    }

    #[test]
    #[ignore]
    fn test_large_script_execution() {
        // Test script execution with large scripts (near the 10KB limit)
        use crate::bitcoin::script::{ByteSequence, Operation, ScriptBuilder};

        println!("Building large script...");
        let mut builder = ScriptBuilder::new();

        // Add many operations to approach the size limit
        // Each OP_DUP is 1 byte
        for _ in 0..9000 {
            builder.add(Operation::OP_DUP);
        }

        // Add some data pushes
        for i in 0..10 {
            let data = vec![i as u8; 100];
            let bytes = ByteSequence::new(bytes::Bytes::from(data));
            builder.add(Operation::OP_PUSH(bytes));
            builder.add(Operation::OP_DROP);
        }

        let script = builder.build().unwrap();
        println!("Script size: {} bytes", script.len());

        // Test script execution
        let start = Instant::now();
        let mut interpreter = ScriptInterpreter::new();

        // Push initial value to the stack
        interpreter
            .main_stack
            .push_back(bytes::Bytes::from(vec![1]));

        let result = interpreter.eval_script(&script);
        let exec_time = start.elapsed();

        println!("Script executed in {:?}", exec_time);
        assert!(result.is_ok(), "Large script should execute without errors");
    }

    #[test]
    #[ignore]
    fn test_massive_merkle_tree() {
        // Test merkle tree with many transactions
        const NUM_TRANSACTIONS: usize = 100_000;

        println!("Generating {} transaction hashes...", NUM_TRANSACTIONS);
        let start = Instant::now();

        let mut tx_hashes = Vec::with_capacity(NUM_TRANSACTIONS);
        for i in 0..NUM_TRANSACTIONS {
            let mut hash_bytes = [0u8; 32];
            hash_bytes[0..8].copy_from_slice(&(i as u64).to_le_bytes());
            tx_hashes.push(Hash { raw: hash_bytes });
        }

        let gen_time = start.elapsed();
        println!("Generated {} hashes in {:?}", NUM_TRANSACTIONS, gen_time);

        // Calculate merkle root
        let calc_start = Instant::now();
        let root = calculate_merkle_root(&tx_hashes).unwrap();
        let calc_time = calc_start.elapsed();
        println!("Merkle root calculated in {:?}", calc_time);

        // Test proof generation for various positions
        let positions = vec![0, NUM_TRANSACTIONS / 2, NUM_TRANSACTIONS - 1];
        for pos in positions {
            let proof_start = Instant::now();
            let proof = build_merkle_proof(&tx_hashes, pos).unwrap();
            let proof_time = proof_start.elapsed();

            let verify_start = Instant::now();
            let is_valid = verify_merkle_proof(&tx_hashes[pos], pos, &proof, &root);
            let verify_time = verify_start.elapsed();

            println!(
                "Position {}: proof size: {}, generation: {:?}, verification: {:?}",
                pos,
                proof.len(),
                proof_time,
                verify_time
            );
            assert!(is_valid);
        }
    }

    #[test]
    #[ignore]
    fn test_varint_stress() {
        // Test varint encoding/decoding with many values
        const NUM_VALUES: usize = 1_000_000;

        println!("Encoding {} varints...", NUM_VALUES);
        let start = Instant::now();

        let mut buffer = BytesMut::new();
        let mut values = Vec::with_capacity(NUM_VALUES);

        // Generate and encode values with various sizes
        for i in 0..NUM_VALUES {
            let value = match i % 4 {
                0 => (i % 253) as u64,         // 1-byte encoding
                1 => (i % 65536) as u64,       // 3-byte encoding
                2 => (i % 0x100000000) as u64, // 5-byte encoding
                3 => i as u64 * 0x100000000,   // 9-byte encoding
                _ => unreachable!(),
            };
            values.push(value);
            varint_encode(&mut buffer, value).unwrap();
        }

        let encode_time = start.elapsed();
        println!(
            "Encoded in {:?}, total size: {} bytes",
            encode_time,
            buffer.len()
        );

        // Decode all values
        let decode_start = Instant::now();
        let mut decoded_values = Vec::with_capacity(NUM_VALUES);

        while buffer.has_remaining() {
            decoded_values.push(varint_decode(&mut buffer).unwrap());
        }

        let decode_time = decode_start.elapsed();
        println!("Decoded in {:?}", decode_time);

        assert_eq!(values.len(), decoded_values.len());
        assert_eq!(values, decoded_values);
    }

    #[test]
    #[ignore]
    fn test_p2p_message_flood() {
        // Test handling many P2P messages
        use crate::p2p::{Message, MessageFramer, MAGIC_MAINNET};

        const NUM_MESSAGES: usize = 10_000;

        println!("Creating {} P2P messages...", NUM_MESSAGES);
        let start = Instant::now();

        let mut framer = MessageFramer::new();
        let mut total_size = 0;

        // Create a mix of different message types
        let messages: Vec<Message> = (0..NUM_MESSAGES)
            .map(|i| match i % 5 {
                0 => Message::Ping(i as u64),
                1 => Message::Pong(i as u64),
                2 => Message::Verack,
                3 => Message::GetAddr,
                4 => Message::SendHeaders,
                _ => unreachable!(),
            })
            .collect();

        // Frame all messages
        let mut framed_data = Vec::new();
        for msg in &messages {
            let framed = framer.frame_message(MAGIC_MAINNET, msg).unwrap();
            total_size += framed.len();
            framed_data.extend_from_slice(framed);
        }

        let frame_time = start.elapsed();
        println!(
            "Framed {} messages in {:?}, total size: {} bytes",
            NUM_MESSAGES, frame_time, total_size
        );

        // Decode all messages
        let decode_start = Instant::now();
        let mut decoded_count = 0;

        framer.add_data(&framed_data);

        while let Some((header, _payload)) = framer.decode_message().unwrap() {
            decoded_count += 1;
            assert_eq!(header.magic, MAGIC_MAINNET);
        }

        let decode_time = decode_start.elapsed();
        println!("Decoded {} messages in {:?}", decoded_count, decode_time);

        assert_eq!(decoded_count, NUM_MESSAGES);
    }

    #[test]
    #[ignore]
    fn test_amount_calculations_stress() {
        // Test many amount calculations
        const NUM_OPERATIONS: usize = 1_000_000;

        println!("Performing {} amount operations...", NUM_OPERATIONS);
        let start = Instant::now();

        let mut total = Amount::ZERO;

        for i in 0..NUM_OPERATIONS {
            let amount = Amount::from_satoshis((i % 1000) as i64);
            total = total + amount;

            // Occasionally subtract to keep total reasonable
            if i % 100 == 0 && total.satoshis > 1000 {
                total = total - Amount::from_satoshis(500);
            }
        }

        let calc_time = start.elapsed();
        println!("Completed {} operations in {:?}", NUM_OPERATIONS, calc_time);
        println!("Final total: {} satoshis", total.satoshis);

        // Verify the total is reasonable
        assert!(total.satoshis < 1_000_000_000);
    }

    #[test]
    #[ignore]
    fn test_script_number_edge_cases_stress() {
        // Test script number conversions with many edge cases
        use num::{BigInt, ToPrimitive};

        const NUM_TESTS: usize = 100_000;

        println!("Testing {} script number conversions...", NUM_TESTS);
        let start = Instant::now();

        // Test values around boundaries
        let test_values: Vec<i64> = (0..NUM_TESTS)
            .map(|i| match i % 10 {
                0 => 0,
                1 => i as i64,
                2 => -(i as i64),
                3 => 127,
                4 => -127,
                5 => 128,
                6 => -128,
                7 => 32767,
                8 => -32767,
                9 => i64::MAX / 1000000,
                _ => unreachable!(),
            })
            .collect();

        let mut errors = 0;
        for value in &test_values {
            let bigint = BigInt::from(*value);
            let bytes = bigint.to_signed_bytes_le();
            let recovered_bigint = BigInt::from_signed_bytes_le(&bytes);
            let recovered = recovered_bigint.to_i64().unwrap_or(i64::MAX);

            if *value != recovered {
                errors += 1;
                if errors < 10 {
                    println!("Mismatch: {} != {} (bytes: {:?})", value, recovered, bytes);
                }
            }
        }

        let test_time = start.elapsed();
        println!(
            "Tested {} values in {:?}, errors: {}",
            NUM_TESTS, test_time, errors
        );

        assert_eq!(errors, 0, "All conversions should be accurate");
    }
}
