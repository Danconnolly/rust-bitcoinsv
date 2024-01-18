use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use hex::{FromHex, ToHex};
use crate::bitcoin::hash::Hash;
use crate::bitcoin::binary::Encodable;
use crate::bitcoin::VarInt;


/// The TxHash is used to identify transactions and ensure immutability.
pub type TxHash = Hash;

/// A Bitcoin transaction.
pub struct Tx {
    /// transaction version number
    pub version: u32,
    pub inputs: Vec<TxInput>,       // inputs
    pub outputs: Vec<TxOutput>,     // outputs
    /// lock time
    pub lock_time: u32,
}

impl Tx {
    pub fn hash(&self) -> Hash {
        let mut v = Vec::new();
        self.encode_into(&mut v).unwrap();
        Hash::sha256d(&v)
    }
}

impl FromHex for Tx {
    type Error = crate::Error;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let mut bytes = hex::decode(hex)?;
        let mut cursor = Cursor::new(&mut bytes);
        let tx = Tx::decode(&mut cursor)?;
        Ok(tx)
    }
}

impl ToHex for Tx {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        let mut bytes = Vec::new();
        self.encode_into(&mut bytes).unwrap();
        bytes.encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        let mut bytes = Vec::new();
        self.encode_into(&mut bytes).unwrap();
        bytes.encode_hex_upper()
    }
}

impl Encodable for Tx {
    fn decode<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<Tx> {
        let version = reader.read_u32::<LittleEndian>()?;
        let num_inputs = VarInt::decode(reader)?;
        let mut inputs = Vec::new();
        for _i in 0..num_inputs.value {
            let input= TxInput::decode(reader)?;
            inputs.push(input);
        }
        let num_outputs = VarInt::decode(reader)?;
        let mut outputs = Vec::new();
        for _i in 0..num_outputs.value {
            let output = TxOutput::decode(reader)?;
            outputs.push(output);
        }
        let lock_time = reader.read_u32::<LittleEndian>()?;
        Ok(Tx {
            version,
            inputs,
            outputs,
            lock_time
        })
    }

    fn encode_into<W: WriteBytesExt + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_u32::<LittleEndian>(self.version)?;
        VarInt::new(self.inputs.len() as u64).encode_into(writer)?;
        for input in self.inputs.iter() {
            input.encode_into(writer)?;
        }
        VarInt::new(self.outputs.len() as u64).encode_into(writer)?;
        for output in self.outputs.iter() {
            output.encode_into(writer)?;
        }
        writer.write_u32::<LittleEndian>(self.lock_time)?;
        Ok(())
    }

    fn size(&self) -> usize {
        let mut sz = VarInt::new(self.inputs.len() as u64).size();
        for input in self.inputs.iter() {
            sz += input.size();
        }
        sz += VarInt::new(self.outputs.len() as u64).size();
        for output in self.outputs.iter() {
            sz += output.size();
        }
        sz + 8
    }
}

/// An Outpoint is a reference to a specific output of a specific transaction.
pub struct Outpoint {
    raw: [u8; 36],
}

impl Outpoint {
    pub const SIZE: usize = 36;

    /// The hash of transaction.
    pub fn tx_hash(&self) -> TxHash {
        TxHash::from(&self.raw[..32])
    }

    /// The index of the output.
    pub fn index(&self) -> u32 {
        u32::from_le_bytes(self.raw[32..36].try_into().unwrap())
    }
}

impl Encodable for Outpoint {
    fn decode<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<Outpoint> {
        let mut outpoint: [u8; 36] = [0; 36];
        reader.read_exact(&mut outpoint)?;
        Ok(Outpoint {
            raw: outpoint,
        })
    }

    fn encode_into<W: WriteBytesExt + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_all(&self.raw)?;
        Ok(())
    }

    fn size(&self) -> usize {
        Outpoint::SIZE
    }
}

/// A TxInput is an input to a transaction.
pub struct TxInput {
    pub outpoint: Outpoint,
    raw_script: Vec<u8>,
    pub sequence: u32,
}

impl Encodable for TxInput {
    fn decode<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<TxInput> {
        let outpoint = Outpoint::decode(reader)?;
        let script_size = VarInt::decode(reader)?;
        let mut script = vec![0u8; script_size.value as usize];
        reader.read_exact(&mut script)?;
        let sequence = reader.read_u32::<LittleEndian>()?;
        Ok(TxInput {
            outpoint,
            raw_script: script,
            sequence,
        })
    }

    fn encode_into<W: WriteBytesExt + Send>(&self, writer: &mut W) -> crate::Result<()> {
        self.outpoint.encode_into(writer)?;
        VarInt::new(self.raw_script.len() as u64).encode_into(writer)?;
        writer.write_all(&self.raw_script)?;
        writer.write_u32::<LittleEndian>(self.sequence)?;
        Ok(())
    }

    fn size(&self) -> usize {
        self.outpoint.size() + VarInt::new(self.raw_script.len() as u64).size() + self.raw_script.len() + 4
    }
}

/// A TxOutput is an output from a transaction.
pub struct TxOutput {
    pub value: u64,
    raw_script: Vec<u8>,
}

impl Encodable for TxOutput {
    fn decode<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<TxOutput> {
        let value = reader.read_u64::<LittleEndian>()?;
        let script_size = VarInt::decode(reader)?;
        let mut script = vec![0u8; script_size.value as usize];
        reader.read_exact(&mut script)?;
        Ok(TxOutput {
            value,
            raw_script: script,
        })
    }

    fn encode_into<W: WriteBytesExt + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_u64::<LittleEndian>(self.value)?;
        VarInt::new(self.raw_script.len() as u64).encode_into(writer)?;
        writer.write_all(&self.raw_script)?;
        Ok(())
    }

    fn size(&self) -> usize {
        8 + VarInt::new(self.raw_script.len() as u64).size() + self.raw_script.len()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use crate::bitcoin::FromHex;
    use crate::bitcoin::hash::Hash;
    use super::*;


    /// Read a transaction from a byte array and check it
    #[test]
    fn tx_read() {
        let (tx_bin, tx_hash) = get_tx1();
        let mut cursor = Cursor::new(&tx_bin);
        let tx = Tx::decode(&mut cursor).unwrap();
        assert_eq!(tx.version, 1);
        assert_eq!(tx.hash(), tx_hash);
        assert_eq!(tx_bin.len(), tx.size());
    }

    /// If the binary is incomplete, we should get an error
    #[test]
    fn read_short() {
        let (tx_bin, _tx_hash) = get_tx1();
        let mut cursor = Cursor::new(&tx_bin[0..200]);
        assert!(Tx::decode(&mut cursor).is_err());
    }

    /// If we supply too many bytes, then the read should succeed and we should have some bytes left over.
    #[test]
    fn tx_long() {
        let (mut tx_bin, tx_hash) = get_tx1();
        tx_bin.append(&mut vec![0u8; 100]);
        let mut cursor = Cursor::new(&tx_bin[0..300]);
        let tx = Tx::decode(&mut cursor).unwrap();
        assert_eq!(cursor.position(), 211);
        // assert_eq!(tx.size, 211);
        assert_eq!(tx.version, 1);
        assert_eq!(tx.hash(), tx_hash);
    }

    #[test]
    fn read_from_hex() {
        let (mut tx_bin, tx_hash) = get_tx1();
        let tx = Tx::decode(&mut Cursor::new(&tx_bin)).unwrap();
        let tx2 = Tx::from_hex(tx.encode_hex::<String>()).unwrap();
        assert_eq!(tx.hash(), tx_hash);
        assert_eq!(tx2.hash(), tx_hash);
    }

    #[test]
    fn check_deser() {
        let (mut tx_bin, tx_hash) = get_tx1();
        let tx = Tx::decode(&mut Cursor::new(&tx_bin)).unwrap();
        assert_eq!(tx.hash(), tx_hash);
        assert_eq!(tx.version, 1);
        assert_eq!(tx.inputs.len(), 1);
        let i = tx.inputs.get(0).unwrap();
        assert_eq!(i.outpoint.tx_hash(), Hash::from("755f816c02d01c9c0a2f80079132d7b05a1891dc0c860afc6b13e27adc2e058a"));
        assert_eq!(i.outpoint.index(), 1);
        assert_eq!(tx.outputs.len(), 2);
    }

    fn get_tx1() -> (Vec<u8>, Hash) {
        let tx_hex = "01000000018a052edc7ae2136bfc0a860cdc91185ab0d7329107802f0a9c1cd0026c815f75010000006b483045022100e587ef1b4497a6694cad646cab468b6ece2fa98c7f49f9488611ca34eecebd1002205c4ea9066484bd1bffb7fdd7d84b5ae0ee6b7cdc20a8a513e41e420e0633b98841210262142850483b6728b8ecd299e4d0c8cf30ea0636f66205166814e52d73b64b4bffffffff0200000000000000000a006a075354554b2e434fb8ce3f01000000001976a91454cba8da8701174e34aac2bb31d42a88e2c302d088ac00000000";
        let tx_hash = "3abc31f8ff40ffb66d9037e156842fe782e6fa1ae728759263471c68660095f1";
        let tx_bin = hex::decode(tx_hex).unwrap();
        (tx_bin, Hash::from_hex(tx_hash).unwrap())
    }
}
