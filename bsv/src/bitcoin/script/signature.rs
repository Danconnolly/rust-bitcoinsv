use crate::bitcoin::{Encodable, Hash, PrivateKey, PublicKey, Script, Tx, TxOutput};
use crate::{Error, Result};
use bytes::{BufMut, Bytes, BytesMut};
use secp256k1::{ecdsa::Signature, Message, Secp256k1};

/// Signature hash types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigHashType {
    /// Sign all inputs and outputs (default)
    All = 0x01,
    /// Sign all inputs, but no outputs
    None = 0x02,
    /// Sign all inputs and the output with the same index
    Single = 0x03,
    /// Can be combined with above types using bitwise OR
    AnyoneCanPay = 0x80,
    /// All | AnyoneCanPay
    AllAnyoneCanPay = 0x81,
    /// None | AnyoneCanPay
    NoneAnyoneCanPay = 0x82,
    /// Single | AnyoneCanPay
    SingleAnyoneCanPay = 0x83,
}

impl SigHashType {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte & 0x1f {
            0x01 => {
                if byte & 0x80 != 0 {
                    Some(SigHashType::AllAnyoneCanPay)
                } else {
                    Some(SigHashType::All)
                }
            }
            0x02 => {
                if byte & 0x80 != 0 {
                    Some(SigHashType::NoneAnyoneCanPay)
                } else {
                    Some(SigHashType::None)
                }
            }
            0x03 => {
                if byte & 0x80 != 0 {
                    Some(SigHashType::SingleAnyoneCanPay)
                } else {
                    Some(SigHashType::Single)
                }
            }
            _ => None,
        }
    }

    pub fn anyone_can_pay(&self) -> bool {
        (*self as u8) & 0x80 != 0
    }
}

/// Calculate the signature hash for a transaction input
pub fn calculate_signature_hash(
    tx: &Tx,
    input_index: usize,
    subscript: &[u8],
    sighash_type: SigHashType,
) -> Result<Hash> {
    if input_index >= tx.inputs.len() {
        return Err(Error::BadArgument("Invalid input index".to_string()));
    }

    // Create a copy of the transaction for modification
    let mut tx_copy = tx.clone();

    // Clear all input scripts
    for input in &mut tx_copy.inputs {
        input.script = Script { raw: Bytes::new() };
    }

    // Set the subscript for the input being signed
    tx_copy.inputs[input_index].script = Script {
        raw: Bytes::copy_from_slice(subscript),
    };

    // Handle different sighash types
    match sighash_type {
        SigHashType::All | SigHashType::AllAnyoneCanPay => {
            // Sign all outputs, no modifications needed
        }
        SigHashType::None | SigHashType::NoneAnyoneCanPay => {
            // Clear all outputs
            tx_copy.outputs.clear();
            // Clear sequence numbers of other inputs
            for (i, input) in tx_copy.inputs.iter_mut().enumerate() {
                if i != input_index {
                    input.sequence = 0;
                }
            }
        }
        SigHashType::Single | SigHashType::SingleAnyoneCanPay => {
            // Check if output exists
            if input_index >= tx_copy.outputs.len() {
                return Err(Error::BadArgument(
                    "No matching output for SIGHASH_SINGLE".to_string(),
                ));
            }

            // Keep only the output at the same index
            let output = tx_copy.outputs[input_index].clone();
            tx_copy.outputs.clear();
            tx_copy.outputs.resize(
                input_index + 1,
                TxOutput {
                    value: u64::MAX,
                    script: Script { raw: Bytes::new() },
                },
            );
            tx_copy.outputs[input_index] = output;

            // Clear sequence numbers of other inputs
            for (i, input) in tx_copy.inputs.iter_mut().enumerate() {
                if i != input_index {
                    input.sequence = 0;
                }
            }
        }
        SigHashType::AnyoneCanPay => {
            // ANYONECANPAY flag alone is invalid
            return Err(Error::BadArgument(
                "ANYONECANPAY must be combined with another sighash type".to_string(),
            ));
        }
    }

    // Handle ANYONECANPAY
    if sighash_type.anyone_can_pay() {
        // Keep only the input being signed
        let input = tx_copy.inputs[input_index].clone();
        tx_copy.inputs.clear();
        tx_copy.inputs.push(input);
    }

    // Serialize the modified transaction
    let mut buffer = BytesMut::with_capacity(tx_copy.encoded_size() as usize + 4);
    tx_copy.to_binary(&mut buffer)?;

    // Append sighash type
    buffer.put_u32_le(sighash_type as u32);

    // Double SHA256
    Ok(Hash::sha256d(&buffer))
}

/// Sign a transaction input
pub fn sign_input(
    tx: &Tx,
    input_index: usize,
    subscript: &[u8],
    private_key: &PrivateKey,
    sighash_type: SigHashType,
) -> Result<Bytes> {
    // Calculate signature hash
    let sighash = calculate_signature_hash(tx, input_index, subscript, sighash_type)?;

    // Create message from hash
    let message = Message::from_digest(sighash.raw);

    // Sign the message
    let secp = Secp256k1::new();
    let signature = secp.sign_ecdsa(message, &private_key.inner);

    // Serialize signature with sighash type
    let mut sig_bytes = signature.serialize_der().to_vec();
    sig_bytes.push(sighash_type as u8);

    Ok(Bytes::from(sig_bytes))
}

/// Verify a signature for a transaction input
pub fn verify_signature(
    sig_bytes: &[u8],
    pubkey_bytes: &[u8],
    tx: &Tx,
    input_index: usize,
    subscript: &[u8],
) -> Result<bool> {
    // Check minimum signature length
    if sig_bytes.is_empty() {
        return Ok(false);
    }

    // Extract sighash type from last byte
    let sighash_byte = sig_bytes[sig_bytes.len() - 1];
    let sighash_type = SigHashType::from_byte(sighash_byte)
        .ok_or_else(|| Error::BadArgument("Invalid sighash type".to_string()))?;

    // Parse signature (without sighash byte)
    let sig_der = &sig_bytes[..sig_bytes.len() - 1];
    let signature = Signature::from_der(sig_der)
        .map_err(|_| Error::BadArgument("Invalid signature format".to_string()))?;

    // Parse public key
    let secp_pubkey = secp256k1::PublicKey::from_slice(pubkey_bytes)
        .map_err(|_| Error::BadArgument("Invalid public key".to_string()))?;
    let pubkey = PublicKey::new(secp_pubkey);

    // Calculate signature hash
    let sighash = calculate_signature_hash(tx, input_index, subscript, sighash_type)?;

    // Create message from hash
    let message = Message::from_digest(sighash.raw);

    // Verify signature
    let secp = Secp256k1::new();
    Ok(secp
        .verify_ecdsa(message, &signature, &pubkey.inner)
        .is_ok())
}

/// Remove signature from script for signature hash calculation
pub fn remove_signature_from_script(script: &[u8], signature: &[u8]) -> Bytes {
    let mut result = BytesMut::new();
    let mut i = 0;

    while i < script.len() {
        // Check if we're at the start of a push operation
        if i + signature.len() <= script.len() {
            let mut found = false;

            // Check for direct push (OP_PUSH)
            if script[i] == signature.len() as u8 && i + 1 + signature.len() <= script.len() {
                if &script[i + 1..i + 1 + signature.len()] == signature {
                    // Skip the push opcode and signature
                    i += 1 + signature.len();
                    found = true;
                }
            }

            if !found {
                result.put_u8(script[i]);
                i += 1;
            }
        } else {
            result.put_u8(script[i]);
            i += 1;
        }
    }

    result.freeze()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::{Outpoint, Script, Tx, TxHash, TxInput, TxOutput};
    use hex::FromHex;
    use hex_literal::hex;

    #[test]
    fn test_sighash_type_parsing() {
        assert_eq!(SigHashType::from_byte(0x01), Some(SigHashType::All));
        assert_eq!(SigHashType::from_byte(0x02), Some(SigHashType::None));
        assert_eq!(SigHashType::from_byte(0x03), Some(SigHashType::Single));
        assert_eq!(
            SigHashType::from_byte(0x81),
            Some(SigHashType::AllAnyoneCanPay)
        );
        assert_eq!(
            SigHashType::from_byte(0x82),
            Some(SigHashType::NoneAnyoneCanPay)
        );
        assert_eq!(
            SigHashType::from_byte(0x83),
            Some(SigHashType::SingleAnyoneCanPay)
        );

        assert!(SigHashType::All.anyone_can_pay() == false);
        assert!(SigHashType::AllAnyoneCanPay.anyone_can_pay() == true);
    }

    fn create_outpoint(tx_hash: &TxHash, index: u32) -> Outpoint {
        let mut raw = BytesMut::with_capacity(36);
        raw.put_slice(&tx_hash.raw);
        raw.put_u32_le(index);
        Outpoint { raw: raw.freeze() }
    }

    #[test]
    fn test_signature_creation_and_verification() {
        // Create a simple transaction
        let prev_hash =
            TxHash::from_hex("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        let tx = Tx {
            version: 1,
            inputs: vec![TxInput {
                outpoint: create_outpoint(&prev_hash, 0),
                script: Script { raw: Bytes::new() },
                sequence: 0xFFFFFFFF,
            }],
            outputs: vec![TxOutput {
                value: 50_000,
                script: Script {
                    raw: Bytes::from(
                        &hex!("76a91489abcdefabbaabbaabbaabbaabbaabbaabbaabba88ac")[..],
                    ),
                },
            }],
            lock_time: 0,
        };

        // Create a private key
        let private_key = PrivateKey::generate();
        let public_key = PublicKey::from(&private_key);

        // Create subscript (previous output's script)
        let subscript = hex!("76a91489abcdefabbaabbaabbaabbaabbaabbaabbaabba88ac");

        // Sign the input
        let signature = sign_input(&tx, 0, &subscript, &private_key, SigHashType::All).unwrap();

        // Verify the signature
        let verified =
            verify_signature(&signature, &public_key.to_bytes(), &tx, 0, &subscript).unwrap();

        assert!(verified);

        // Test with wrong public key
        let wrong_private_key = PrivateKey::generate();
        let wrong_public_key = PublicKey::from(&wrong_private_key);

        let verified_wrong =
            verify_signature(&signature, &wrong_public_key.to_bytes(), &tx, 0, &subscript).unwrap();

        assert!(!verified_wrong);
    }

    #[test]
    fn test_remove_signature_from_script() {
        // Script with signature: <sig> <pubkey>
        let sig = hex!("304402201234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef02201234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef01");
        let pubkey = hex!("021234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");

        let mut script = BytesMut::new();
        script.put_u8(sig.len() as u8);
        script.put_slice(&sig);
        script.put_u8(pubkey.len() as u8);
        script.put_slice(&pubkey);

        let result = remove_signature_from_script(&script, &sig);

        // Should only have the pubkey push left
        assert_eq!(result.len(), 1 + pubkey.len());
        assert_eq!(result[0], pubkey.len() as u8);
        assert_eq!(&result[1..], &pubkey);
    }

    #[test]
    fn test_different_sighash_types() {
        let prev_hash1 =
            TxHash::from_hex("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap();
        let prev_hash2 =
            TxHash::from_hex("0000000000000000000000000000000000000000000000000000000000000002")
                .unwrap();

        let tx = Tx {
            version: 1,
            inputs: vec![
                TxInput {
                    outpoint: create_outpoint(&prev_hash1, 0),
                    script: Script { raw: Bytes::new() },
                    sequence: 0xFFFFFFFF,
                },
                TxInput {
                    outpoint: create_outpoint(&prev_hash2, 0),
                    script: Script { raw: Bytes::new() },
                    sequence: 0xFFFFFFFF,
                },
            ],
            outputs: vec![
                TxOutput {
                    value: 25_000,
                    script: Script {
                        raw: Bytes::from(
                            &hex!("76a91489abcdefabbaabbaabbaabbaabbaabbaabbaabba88ac")[..],
                        ),
                    },
                },
                TxOutput {
                    value: 25_000,
                    script: Script {
                        raw: Bytes::from(
                            &hex!("76a914fedcba9876543210fedcba9876543210fedcba9888ac")[..],
                        ),
                    },
                },
            ],
            lock_time: 0,
        };

        let subscript = hex!("76a91489abcdefabbaabbaabbaabbaabbaabbaabbaabba88ac");

        // Test different sighash types produce different hashes
        let hash_all = calculate_signature_hash(&tx, 0, &subscript, SigHashType::All).unwrap();
        let hash_none = calculate_signature_hash(&tx, 0, &subscript, SigHashType::None).unwrap();
        let hash_single =
            calculate_signature_hash(&tx, 0, &subscript, SigHashType::Single).unwrap();
        let hash_all_anyonecanpay =
            calculate_signature_hash(&tx, 0, &subscript, SigHashType::AllAnyoneCanPay).unwrap();

        assert_ne!(hash_all, hash_none);
        assert_ne!(hash_all, hash_single);
        assert_ne!(hash_all, hash_all_anyonecanpay);
        assert_ne!(hash_none, hash_single);
    }
}
