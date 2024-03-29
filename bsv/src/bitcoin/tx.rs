use async_trait::async_trait;
use hex::{FromHex, ToHex};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::hash::Hash;
use crate::bitcoin::{Encodable, varint_decode, varint_encode, varint_size};


/// The TxHash is used to identify transactions.
pub type TxHash = Hash;

/// A Bitcoin transaction.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
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
        let v = self.to_binary_buf().unwrap();
        Hash::sha256d(&v)
    }
}

impl FromHex for Tx {
    type Error = crate::Error;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let bytes = hex::decode(hex)?;
        let tx = Tx::from_binary_buf(&mut bytes.as_slice())?;
        Ok(tx)
    }
}

impl ToHex for Tx {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        let bytes = self.to_binary_buf().unwrap();
        bytes.encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        let bytes = self.to_binary_buf().unwrap();
        bytes.encode_hex_upper()
    }
}


#[async_trait]
impl Encodable for Tx {
    async fn from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let version = reader.read_u32_le().await?;
        let num_inputs = varint_decode(reader).await?;
        // todo: check size before allocation
        let mut inputs = Vec::with_capacity(num_inputs as usize);
        for _i in 0..num_inputs {
            let input= TxInput::from_binary(reader).await?;
            inputs.push(input);
        }
        let num_outputs = varint_decode(reader).await?;
        // todo: check size before allocation
        let mut outputs = Vec::with_capacity(num_outputs as usize);
        for _i in 0..num_outputs {
            let output = TxOutput::from_binary(reader).await?;
            outputs.push(output);
        }
        let lock_time = reader.read_u32_le().await?;
        Ok(Tx {
            version,
            inputs,
            outputs,
            lock_time
        })
    }

    async fn to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_u32_le(self.version).await?;
        varint_encode(writer, self.inputs.len() as u64).await?;
        for input in self.inputs.iter() {
            input.to_binary(writer).await?;
        }
        varint_encode(writer, self.outputs.len() as u64).await?;
        for output in self.outputs.iter() {
            output.to_binary(writer).await?;
        }
        writer.write_u32_le(self.lock_time).await?;
        Ok(())
    }

    fn size(&self) -> usize {
        let mut sz = varint_size(self.inputs.len() as u64);
        for input in self.inputs.iter() {
            sz += input.size();
        }
        sz += varint_size(self.outputs.len() as u64);
        for output in self.outputs.iter() {
            sz += output.size();
        }
        sz + 8
    }
}

/// An Outpoint is a reference to a specific output of a specific transaction.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Outpoint {
    pub tx_hash: Hash,
    pub index: u32,
}

impl Outpoint {
    pub const SIZE: usize = 36;
}

#[async_trait]
impl Encodable for Outpoint {
    async fn from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let tx_hash = Hash::from_binary(reader).await?;
        let index = reader.read_u32_le().await?;
        Ok(Outpoint {
            tx_hash, index,
        })
    }

    async fn to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        self.tx_hash.to_binary(writer).await?;
        writer.write_u32_le(self.index).await?;
        Ok(())
    }

    fn size(&self) -> usize {
        Outpoint::SIZE
    }
}

/// A TxInput is an input to a transaction.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TxInput {
    pub outpoint: Outpoint,
    pub raw_script: Vec<u8>,
    pub sequence: u32,
}

#[async_trait]
impl Encodable for TxInput {
    async fn from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let outpoint = Outpoint::from_binary(reader).await?;
        let script_size = varint_decode(reader).await?;
        // todo: check size before allocation
        let mut script = vec![0u8; script_size as usize];
        reader.read_exact(&mut script).await?;
        let sequence = reader.read_u32_le().await?;
        Ok(TxInput {
            outpoint,
            raw_script: script,
            sequence,
        })
    }

    async fn to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        self.outpoint.to_binary(writer).await?;
        varint_encode(writer, self.raw_script.len() as u64).await?;
        writer.write_all(&self.raw_script).await?;
        writer.write_u32_le(self.sequence).await?;
        Ok(())
    }

    fn size(&self) -> usize {
        self.outpoint.size() + varint_size(self.raw_script.len() as u64) + self.raw_script.len() + 4
    }
}

/// A TxOutput is an output from a transaction.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TxOutput {
    pub value: u64,
    pub raw_script: Vec<u8>,
}

#[async_trait]
impl Encodable for TxOutput {
    async fn from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let value = reader.read_u64_le().await?;
        let script_size = varint_decode(reader).await?;
        // todo: check size before allocation?
        let mut script = vec![0u8; script_size as usize];
        reader.read_exact(&mut script).await?;
        Ok(TxOutput {
            value,
            raw_script: script,
        })
    }

    async fn to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_u64_le(self.value).await?;
        varint_encode(writer, self.raw_script.len() as u64).await?;
        writer.write_all(&self.raw_script).await?;
        Ok(())
    }

    fn size(&self) -> usize {
        8 + varint_size(self.raw_script.len() as u64) + self.raw_script.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::bitcoin::FromHex;
    use crate::bitcoin::hash::Hash;
    use super::*;


    /// Read a transaction from a byte array and check it
    #[test]
    fn tx_read() {
        let (tx_bin, tx_hash) = get_tx1();
        let tx = Tx::from_binary_buf(tx_bin.as_slice()).unwrap();
        assert_eq!(tx.version, 1);
        assert_eq!(tx.hash(), tx_hash);
        assert_eq!(tx_bin.len(), tx.size());
    }

    /// If the binary is incomplete, we should get an error
    #[test]
    fn read_short() {
        let (tx_bin, _tx_hash) = get_tx1();
        assert!(Tx::from_binary_buf(tx_bin[0..200].iter().as_slice()).is_err());
    }

    /// If we supply too many bytes then the read should succeed and we should have some bytes left over.
    #[test]
    fn tx_long() {
        let (mut tx_bin, tx_hash) = get_tx1();
        tx_bin.append(&mut vec![0u8; 100]);
        let tx = Tx::from_binary_buf(tx_bin.as_slice()).unwrap();
        assert_eq!(tx.size(), 211);
        assert_eq!(tx.version, 1);
        assert_eq!(tx.hash(), tx_hash);
    }

    #[test]
    fn read_from_hex() {
        let (tx_bin, tx_hash) = get_tx1();
        let tx = Tx::from_binary_buf(tx_bin.as_slice()).unwrap();
        let tx2 = Tx::from_hex(tx.encode_hex::<String>()).unwrap();
        assert_eq!(tx.hash(), tx_hash);
        assert_eq!(tx2.hash(), tx_hash);
    }

    #[test]
    fn check_deser() {
        let (tx_bin, tx_hash) = get_tx1();
        let tx = Tx::from_binary_buf(tx_bin.as_slice()).unwrap();
        assert_eq!(tx.hash(), tx_hash);
        assert_eq!(tx.version, 1);
        assert_eq!(tx.inputs.len(), 1);
        let i = tx.inputs.get(0).unwrap();
        assert_eq!(i.outpoint.tx_hash, Hash::from("755f816c02d01c9c0a2f80079132d7b05a1891dc0c860afc6b13e27adc2e058a"));
        assert_eq!(i.outpoint.index, 1);
        assert_eq!(tx.outputs.len(), 2);
    }

    fn get_tx1() -> (Vec<u8>, Hash) {
        let tx_hex = "01000000018a052edc7ae2136bfc0a860cdc91185ab0d7329107802f0a9c1cd0026c815f75010000006b483045022100e587ef1b4497a6694cad646cab468b6ece2fa98c7f49f9488611ca34eecebd1002205c4ea9066484bd1bffb7fdd7d84b5ae0ee6b7cdc20a8a513e41e420e0633b98841210262142850483b6728b8ecd299e4d0c8cf30ea0636f66205166814e52d73b64b4bffffffff0200000000000000000a006a075354554b2e434fb8ce3f01000000001976a91454cba8da8701174e34aac2bb31d42a88e2c302d088ac00000000";
        let tx_hash = "3abc31f8ff40ffb66d9037e156842fe782e6fa1ae728759263471c68660095f1";
        let tx_bin = hex::decode(tx_hex).unwrap();
        (tx_bin, Hash::from_hex(tx_hash).unwrap())
    }
}
