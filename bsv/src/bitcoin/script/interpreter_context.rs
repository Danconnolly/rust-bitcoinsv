use crate::bitcoin::script::{verify_script, Script};
use crate::bitcoin::Tx;
use crate::Result;

/// Verify a script with full transaction context (wrapper for compatibility)
pub fn verify_script_with_context(
    script_sig: &Script,
    script_pubkey: &Script,
    tx: &Tx,
    input_index: usize,
) -> Result<bool> {
    verify_script(script_sig, script_pubkey, tx, input_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::script::{ByteSequence, Operation, ScriptBuilder};
    use crate::bitcoin::{Hash, Outpoint, PrivateKey, PublicKey, Script, Tx, TxInput, TxOutput};
    use bytes::{BufMut, Bytes};
    use hex::FromHex;
    use hex_literal::hex;

    fn create_outpoint(tx_hash: &Hash, index: u32) -> Outpoint {
        let mut raw = bytes::BytesMut::with_capacity(36);
        raw.put_slice(&tx_hash.raw);
        raw.put_u32_le(index);
        Outpoint { raw: raw.freeze() }
    }

    #[test]
    fn test_p2pk_script_verification() {
        // Create a private key and public key
        let private_key = PrivateKey::generate();
        let public_key = PublicKey::from(&private_key);

        // Create a P2PK output script: <pubkey> OP_CHECKSIG
        let pubkey_bytes = ByteSequence::new(Bytes::from(public_key.to_bytes()));
        let script_pubkey = ScriptBuilder::new()
            .add(Operation::OP_PUSH(pubkey_bytes))
            .add(Operation::OP_CHECKSIG)
            .build()
            .unwrap();

        // Create a transaction to spend
        let prev_hash =
            Hash::from_hex("0000000000000000000000000000000000000000000000000000000000000000")
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

        // Sign the transaction
        use crate::bitcoin::script::{sign_input, SigHashType};
        let signature =
            sign_input(&tx, 0, &script_pubkey.raw, &private_key, SigHashType::All).unwrap();

        // Create the signature script: <sig>
        let sig_bytes = ByteSequence::new(signature);
        let script_sig = ScriptBuilder::new()
            .add(Operation::OP_PUSH(sig_bytes))
            .build()
            .unwrap();

        // Verify the script
        let result = verify_script_with_context(&script_sig, &script_pubkey, &tx, 0).unwrap();
        assert!(result);
    }

    #[test]
    fn test_p2pkh_script_verification() {
        // Create a private key and public key
        let private_key = PrivateKey::generate();
        let public_key = PublicKey::from(&private_key);
        let pubkey_hash = public_key.pubkey_hash();

        // Create a P2PKH output script: OP_DUP OP_HASH160 <pubkeyhash> OP_EQUALVERIFY OP_CHECKSIG
        let pubkey_hash_bytes = ByteSequence::new(Bytes::copy_from_slice(&pubkey_hash.hash));
        let script_pubkey = ScriptBuilder::new()
            .add(Operation::OP_DUP)
            .add(Operation::OP_HASH160)
            .add(Operation::OP_PUSH(pubkey_hash_bytes))
            .add(Operation::OP_EQUALVERIFY)
            .add(Operation::OP_CHECKSIG)
            .build()
            .unwrap();

        // Create a transaction to spend
        let prev_hash =
            Hash::from_hex("0000000000000000000000000000000000000000000000000000000000000000")
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

        // Sign the transaction
        use crate::bitcoin::script::{sign_input, SigHashType};
        let signature =
            sign_input(&tx, 0, &script_pubkey.raw, &private_key, SigHashType::All).unwrap();

        // Create the signature script: <sig> <pubkey>
        let sig_bytes = ByteSequence::new(signature);
        let pubkey_bytes = ByteSequence::new(Bytes::from(public_key.to_bytes()));
        let script_sig = ScriptBuilder::new()
            .add(Operation::OP_PUSH(sig_bytes))
            .add(Operation::OP_PUSH(pubkey_bytes))
            .build()
            .unwrap();

        // Verify the script
        let result = verify_script_with_context(&script_sig, &script_pubkey, &tx, 0).unwrap();
        assert!(result);
    }

    #[test]
    fn test_invalid_signature() {
        // Create keys
        let private_key1 = PrivateKey::generate();
        let public_key1 = PublicKey::from(&private_key1);
        let private_key2 = PrivateKey::generate();
        let _public_key2 = PublicKey::from(&private_key2);

        // Create a P2PK output script with public_key1
        let pubkey_bytes = ByteSequence::new(Bytes::from(public_key1.to_bytes()));
        let script_pubkey = ScriptBuilder::new()
            .add(Operation::OP_PUSH(pubkey_bytes))
            .add(Operation::OP_CHECKSIG)
            .build()
            .unwrap();

        // Create a transaction
        let prev_hash =
            Hash::from_hex("0000000000000000000000000000000000000000000000000000000000000000")
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

        // Sign with private_key2 (wrong key)
        use crate::bitcoin::script::{sign_input, SigHashType};
        let signature =
            sign_input(&tx, 0, &script_pubkey.raw, &private_key2, SigHashType::All).unwrap();

        // Create the signature script
        let sig_bytes = ByteSequence::new(signature);
        let script_sig = ScriptBuilder::new()
            .add(Operation::OP_PUSH(sig_bytes))
            .build()
            .unwrap();

        // Verify the script - should fail
        let result = verify_script_with_context(&script_sig, &script_pubkey, &tx, 0).unwrap();
        assert!(!result);
    }
}
