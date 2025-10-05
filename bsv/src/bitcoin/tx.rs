use crate::bitcoin::hash::Hash;
use crate::bitcoin::{varint_decode, varint_encode, varint_size};
use crate::bitcoin::{Encodable, Script};
use crate::Error;
use bytes::{Buf, BufMut, Bytes};
use hex::{FromHex, ToHex};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

/// The TxHash is used to identify transactions.
pub type TxHash = Hash;

/// Maximum number of transaction inputs allowed.
/// Conservative limit: assuming minimum input size of ~41 bytes,
/// and maximum transaction size of 10MB (policy), this allows ~250k inputs.
/// We set a more conservative limit to prevent memory exhaustion attacks.
const MAX_TX_INPUTS: u64 = 100_000;

/// Maximum number of transaction outputs allowed.
/// Conservative limit: assuming minimum output size of ~9 bytes,
/// and maximum transaction size of 10MB (policy), this allows ~1M outputs.
/// We set a more conservative limit to prevent memory exhaustion attacks.
const MAX_TX_OUTPUTS: u64 = 100_000;

/// A Bitcoin transaction.
///
/// This implementation stores the encoded form and extracts fields when they are requested.
#[derive(PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct Tx {
    /// transaction version number
    pub version: u32,
    /// Vector of inputs.
    pub inputs: Vec<TxInput>,
    /// Vector of outputs.
    pub outputs: Vec<TxOutput>,
    /// lock time
    pub lock_time: u32,
}

impl Tx {
    pub fn hash(&self) -> Hash {
        let mut v = Vec::with_capacity(self.encoded_size() as usize);
        self.to_binary(&mut v)
            .expect("Failed to serialize transaction to binary");
        Hash::sha256d(&v)
    }
}

impl FromHex for Tx {
    type Error = Error;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let mut bytes = Bytes::from(hex::decode(hex)?);
        let tx = Tx::from_binary(&mut bytes)?;
        Ok(tx)
    }
}

impl ToHex for Tx {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        let mut v = Vec::with_capacity(self.encoded_size() as usize);
        self.to_binary(&mut v)
            .expect("Failed to serialize transaction to hex");
        v.encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        let mut v = Vec::with_capacity(self.encoded_size() as usize);
        self.to_binary(&mut v)
            .expect("Failed to serialize transaction to hex uppercase");
        v.encode_hex_upper()
    }
}

impl Encodable for Tx {
    fn from_binary(buffer: &mut dyn Buf) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let version = buffer.get_u32_le();
        let num_inputs = varint_decode(buffer)?;

        // Validate number of inputs before allocation to prevent memory exhaustion
        if num_inputs > MAX_TX_INPUTS {
            return Err(Error::BadData(format!(
                "Too many transaction inputs: {} (max: {})",
                num_inputs, MAX_TX_INPUTS
            )));
        }

        let mut inputs = Vec::with_capacity(num_inputs as usize);
        for _ in 0..num_inputs {
            inputs.push(TxInput::from_binary(buffer)?);
        }
        let num_outputs = varint_decode(buffer)?;

        // Validate number of outputs before allocation to prevent memory exhaustion
        if num_outputs > MAX_TX_OUTPUTS {
            return Err(Error::BadData(format!(
                "Too many transaction outputs: {} (max: {})",
                num_outputs, MAX_TX_OUTPUTS
            )));
        }

        let mut outputs = Vec::with_capacity(num_outputs as usize);
        for _ in 0..num_outputs {
            outputs.push(TxOutput::from_binary(buffer)?);
        }
        let lock_time = buffer.get_u32_le();
        Ok(Tx {
            version,
            inputs,
            outputs,
            lock_time,
        })
    }

    fn to_binary(&self, buffer: &mut dyn BufMut) -> crate::Result<()> {
        buffer.put_u32_le(self.version);
        varint_encode(buffer, self.inputs.len() as u64)?;
        for input in &self.inputs {
            input.to_binary(buffer)?;
        }
        varint_encode(buffer, self.outputs.len() as u64)?;
        for output in &self.outputs {
            output.to_binary(buffer)?;
        }
        buffer.put_u32_le(self.lock_time);
        Ok(())
    }

    fn encoded_size(&self) -> u64 {
        let mut size =
            varint_size(self.inputs.len() as u64) + varint_size(self.outputs.len() as u64);
        for input in self.inputs.iter() {
            size += input.encoded_size();
        }
        for output in self.outputs.iter() {
            size += output.encoded_size();
        }
        size + 8
    }
}

/// An Outpoint is a reference to a specific output of a specific transaction.
#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Outpoint {
    pub raw: Bytes,
}

impl Outpoint {
    pub const SIZE: u64 = 36;

    pub fn tx_hash(&self) -> TxHash {
        let slice = &self.raw[0..32];
        TxHash::from(slice)
    }

    pub fn index(&self) -> u32 {
        let mut slice = &self.raw[32..36];
        slice.get_u32_le()
    }
}

impl Encodable for Outpoint {
    fn from_binary(buffer: &mut dyn Buf) -> crate::Result<Self>
    where
        Self: Sized,
    {
        if buffer.remaining() < Self::SIZE as usize {
            Err(Error::DataTooSmall)
        } else {
            Ok(Self {
                raw: buffer.copy_to_bytes(Self::SIZE as usize),
            })
        }
    }

    fn to_binary(&self, buffer: &mut dyn BufMut) -> crate::Result<()> {
        buffer.put_slice(&self.raw);
        Ok(())
    }

    fn encoded_size(&self) -> u64 {
        Self::SIZE
    }
}

impl Debug for Outpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outpoint")
            .field("tx_hash", &format!("{}", self.tx_hash()))
            .field("index", &self.index())
            .finish()
    }
}

/// A TxInput is an input to a transaction.
#[derive(PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct TxInput {
    pub outpoint: Outpoint,
    pub script: Script,
    pub sequence: u32,
}

impl Encodable for TxInput {
    fn from_binary(buffer: &mut dyn Buf) -> crate::Result<Self> {
        let outpoint = Outpoint::from_binary(buffer)?;
        let script = Script::from_binary(buffer)?;
        let sequence = buffer.try_get_u32_le()?;
        Ok(TxInput {
            outpoint,
            script,
            sequence,
        })
    }

    fn to_binary(&self, buffer: &mut dyn BufMut) -> crate::Result<()> {
        self.outpoint.to_binary(buffer)?;
        self.script.to_binary(buffer)?;
        buffer.put_u32_le(self.sequence);
        Ok(())
    }

    fn encoded_size(&self) -> u64 {
        self.outpoint.encoded_size() + self.script.encoded_size() + 4
    }
}

/// A TxOutput is an output from a transaction.
#[derive(PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct TxOutput {
    pub value: u64,
    pub script: Script,
}

impl TxOutput {
    /// Simple new function.
    pub fn new(value: u64, script: Script) -> TxOutput {
        TxOutput { value, script }
    }
}

impl Encodable for TxOutput {
    fn from_binary(buffer: &mut dyn Buf) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let value = buffer.try_get_u64_le()?;
        let script = Script::from_binary(buffer)?;
        Ok(TxOutput { value, script })
    }

    fn to_binary(&self, buffer: &mut dyn BufMut) -> crate::Result<()> {
        buffer.put_u64_le(self.value);
        self.script.to_binary(buffer)?;
        Ok(())
    }

    fn encoded_size(&self) -> u64 {
        self.script.encoded_size() + 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::hash::Hash;
    use crate::bitcoin::FromHex;

    /// Read a transaction from a byte array and check it
    #[test]
    fn tx_read() {
        let (tx_bin, tx_hash) = get_tx1();
        let l = tx_bin.len() as u64;
        let mut bytes = Bytes::from(tx_bin);
        let tx =
            Tx::from_binary(&mut bytes).expect("Failed to parse transaction from binary for test");
        assert_eq!(tx.version, 1);
        assert_eq!(tx.hash(), tx_hash);
        assert_eq!(l, tx.encoded_size());
    }

    /// If the binary is incomplete, we should get an error
    #[test]
    fn read_short() {
        let (tx_bin, _tx_hash) = get_tx1();
        let mut bytes = Bytes::from(tx_bin);
        let mut b2 = bytes.split_to(200);
        assert!(Tx::from_binary(&mut b2).is_err());
    }

    /// If we supply too many bytes then the read should succeed and we should have some bytes left over.
    #[test]
    fn tx_long() {
        let (mut tx_bin, tx_hash) = get_tx1();
        tx_bin.append(&mut vec![0u8; 100]);
        let mut bytes = Bytes::from(tx_bin);
        let tx =
            Tx::from_binary(&mut bytes).expect("Failed to parse transaction from binary for test");
        assert_eq!(tx.encoded_size(), 211);
        assert_eq!(tx.version, 1);
        assert_eq!(tx.hash(), tx_hash);
    }

    #[test]
    fn read_from_hex() {
        let (tx_bin, tx_hash) = get_tx1();
        let mut bytes = Bytes::from(tx_bin);
        let tx =
            Tx::from_binary(&mut bytes).expect("Failed to parse transaction from binary for test");
        let tx2 = Tx::from_hex(tx.encode_hex::<String>())
            .expect("Failed to parse transaction from hex string");
        assert_eq!(tx.hash(), tx_hash);
        assert_eq!(tx2.hash(), tx_hash);
    }

    #[test]
    fn check_deser() {
        let (tx_bin, tx_hash) = get_tx1();
        let mut bytes = Bytes::from(tx_bin);
        let tx =
            Tx::from_binary(&mut bytes).expect("Failed to parse transaction from binary for test");
        assert_eq!(tx.hash(), tx_hash);
        assert_eq!(tx.version, 1);
        assert_eq!(tx.inputs.len(), 1);
        let i = tx
            .inputs
            .first()
            .expect("Transaction should have at least one input");
        assert_eq!(
            i.outpoint.tx_hash(),
            Hash::from_hex("755f816c02d01c9c0a2f80079132d7b05a1891dc0c860afc6b13e27adc2e058a")
                .expect("Valid hash for test")
        );
        assert_eq!(i.outpoint.index(), 1);
        assert_eq!(tx.outputs.len(), 2);
    }

    /// Test Rust standard serde of transaction and sub-structs.
    #[test]
    fn test_bincode() {
        let config = bincode::config::legacy();
        let (tx_bin, tx_hash) = get_tx1();
        let mut bytes = Bytes::from(tx_bin);
        let tx =
            Tx::from_binary(&mut bytes).expect("Failed to parse transaction from binary for test");
        let e = bincode::serde::encode_to_vec(&tx, config)
            .expect("Failed to encode transaction with bincode");
        let (tx2, _): (Tx, usize) = bincode::serde::decode_from_slice(&e, config)
            .expect("Failed to decode transaction with bincode");
        assert_eq!(tx.hash(), tx_hash);
        assert_eq!(tx2.hash(), tx_hash);
    }

    fn get_tx1() -> (Vec<u8>, Hash) {
        let tx_hex = "01000000018a052edc7ae2136bfc0a860cdc91185ab0d7329107802f0a9c1cd0026c815f75010000006b483045022100e587ef1b4497a6694cad646cab468b6ece2fa98c7f49f9488611ca34eecebd1002205c4ea9066484bd1bffb7fdd7d84b5ae0ee6b7cdc20a8a513e41e420e0633b98841210262142850483b6728b8ecd299e4d0c8cf30ea0636f66205166814e52d73b64b4bffffffff0200000000000000000a006a075354554b2e434fb8ce3f01000000001976a91454cba8da8701174e34aac2bb31d42a88e2c302d088ac00000000";
        let tx_hash = "3abc31f8ff40ffb66d9037e156842fe782e6fa1ae728759263471c68660095f1";
        let tx_bin = hex::decode(tx_hex).expect("Failed to decode hex string for test transaction");
        (
            tx_bin,
            Hash::from_hex(tx_hash).expect("Failed to parse hash from hex for test"),
        )
    }

    /// Test that excessive number of inputs triggers validation error
    #[test]
    fn test_excessive_inputs_validation() {
        use crate::bitcoin::varint_encode;

        // Create malicious transaction data with excessive inputs
        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_le_bytes()); // version

        // Add a very large number of inputs (exceeding MAX_TX_INPUTS)
        let excessive_inputs = super::MAX_TX_INPUTS + 1;
        varint_encode(&mut data, excessive_inputs).expect("Failed to encode varint for test");

        // Try to parse this malicious data
        let mut bytes = Bytes::from(data);
        let result = Tx::from_binary(&mut bytes);

        // Should fail with a BadData error
        assert!(result.is_err());
        match result.expect_err("Parsing should fail with validation error") {
            Error::BadData(msg) => {
                assert!(msg.contains("Too many transaction inputs"));
                assert!(msg.contains(&excessive_inputs.to_string()));
            }
            _ => panic!("Expected BadData error for excessive inputs"),
        }
    }

    /// Test that excessive number of outputs triggers validation error
    #[test]
    fn test_excessive_outputs_validation() {
        use crate::bitcoin::varint_encode;

        // Create transaction data with valid inputs but excessive outputs
        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_le_bytes()); // version

        // Add zero inputs
        varint_encode(&mut data, 0u64).expect("Failed to encode zero inputs for test");

        // Add a very large number of outputs (exceeding MAX_TX_OUTPUTS)
        let excessive_outputs = super::MAX_TX_OUTPUTS + 1;
        varint_encode(&mut data, excessive_outputs)
            .expect("Failed to encode excessive outputs for test");

        // Try to parse this malicious data
        let mut bytes = Bytes::from(data);
        let result = Tx::from_binary(&mut bytes);

        // Should fail with a BadData error
        assert!(result.is_err());
        if let Error::BadData(msg) = result.expect_err("Parsing should fail with validation error")
        {
            assert!(msg.contains("Too many transaction outputs"));
            assert!(msg.contains(&excessive_outputs.to_string()));
        } else {
            panic!("Expected BadData error for excessive outputs")
        }
    }

    /// Test that transaction with maximum allowed inputs/outputs succeeds
    #[test]
    fn test_max_allowed_inputs_outputs() {
        use crate::bitcoin::varint_encode;

        // Create transaction data with exactly MAX_TX_INPUTS inputs
        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_le_bytes()); // version

        // Add exactly MAX_TX_INPUTS (this should be allowed)
        varint_encode(&mut data, super::MAX_TX_INPUTS)
            .expect("Failed to encode max inputs for test");

        // We don't actually need to add the input data for this test,
        // as the validation happens before reading the actual inputs.
        // The parsing will fail later due to insufficient data, but not
        // due to the validation check we're testing.

        let mut bytes = Bytes::from(data);
        let result = Tx::from_binary(&mut bytes);

        // Should fail due to insufficient data (not enough bytes for inputs),
        // but NOT due to validation error
        assert!(result.is_err());
        if let Error::BadData(msg) = result.expect_err("Parsing should fail with validation error")
        {
            panic!(
                "Should not fail validation with exactly MAX_TX_INPUTS: {}",
                msg
            )
        }
    }
}
