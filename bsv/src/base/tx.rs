use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::base::{Hash, VarInt};

pub type TxHash = Hash;

/// A Bitcoin transaction.
/// Often, when we read a transaction we dont know in advance how large it is and we have to
/// parse through the transaction discovering its length as we go. This involves several memory
/// allocations as we determine the size of each input and output script and read them in.
/// So we might as well keep these around for later use and only serialize them to a single
/// byte array when necessary.
/// If we knew the size of the transaction in advance, we could allocate a single buffer and read
/// it into that buffer in one go. This would be much faster and use less memory, but it is not the
/// normal case.
pub struct Tx {
    /// transaction version number
    pub version: u32,
    /// the size of the serialized transaction
    pub size: usize,
    inputs: Vec<TxInput>,       // inputs
    outputs: Vec<TxOutput>,     // outputs
    /// lock time
    pub lock_time: u32,
}

impl Tx {
    pub async fn hash(&self) -> Hash {
        return Hash::sha256d(&self.serialize().await);
    }

    pub async fn serialize(&self) -> Vec<u8> {
        let r = vec![0u8; self.size];
        let mut cursor = io::Cursor::new(r);
        self.write(&mut cursor).await.unwrap();
        return cursor.get_ref().clone();
    }

    pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Tx> {
        // our constraints are primarily memory size, not CPU time. We want to read the entire
        // transaction in as quickly as possible, keeping the memory footprint as small as possible.
        let version = reader.read_u32_le().await?;
        let num_inputs = VarInt::read(reader).await?;
        let mut len = 4 + num_inputs.size();
        let mut inputs = Vec::new();
        for _i in 0..num_inputs.value {
            let (input, input_len) = TxInput::read(reader).await?;
            len += input_len;
            inputs.push(input);
        }
        let num_outputs = VarInt::read(reader).await?;
        len += num_outputs.size();
        let mut outputs = Vec::new();
        for _i in 0..num_outputs.value {
            let (output, output_len) = TxOutput::read(reader).await?;
            len += output_len;
            outputs.push(output);
        }
        let lock_time = reader.read_u32_le().await?;
        len += 4;
        return Ok(Tx {
            version,
            size: len,
            inputs,
            outputs,
            lock_time
        });
    }

    pub async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u32_le(self.version).await?;
        VarInt::new(self.inputs.len() as u64).write(writer).await?;
        for input in self.inputs.iter() {
            input.write(writer).await?;
        }
        VarInt::new(self.outputs.len() as u64).write(writer).await?;
        for output in self.outputs.iter() {
            output.write(writer).await?;
        }
        writer.write_u32_le(self.lock_time).await?;
        Ok(())
    }
}

pub struct Outpoint {
    raw: [u8; 36],
}

impl Outpoint {
    pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Outpoint> {
        let mut outpoint: [u8; 36] = [0; 36];
        let _bytes_read = reader.read_exact(&mut outpoint).await?;
        assert_eq!(_bytes_read, 36);
        return Ok(Outpoint {
            raw: outpoint,
        });
    }

    pub async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.raw).await?;
        Ok(())
    }
}

pub struct TxInput {
    pub outpoint: Outpoint,
    raw_script: Vec<u8>,
    pub sequence: u32,
}

impl TxInput {
    pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<(TxInput, usize)> {
        let outpoint = Outpoint::read(reader).await?;
        let script_size = VarInt::read(reader).await?;
        let mut script = vec![0u8; script_size.value as usize];
        let _bytes_read = reader.read_exact(&mut script).await?;
        assert_eq!(_bytes_read, script_size.value as usize);
        let sequence = reader.read_u32_le().await?;
        return Ok((TxInput {
            outpoint,
            raw_script: script,
            sequence,
        }, 40 + script_size.size() + script_size.value as usize));
    }

    pub async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        self.outpoint.write(writer).await?;
        VarInt::new(self.raw_script.len() as u64).write(writer).await?;
        writer.write_all(&self.raw_script).await?;
        writer.write_u32_le(self.sequence).await?;
        Ok(())
    }
}

pub struct TxOutput {
    pub value: u64,
    raw_script: Vec<u8>,
}

impl TxOutput {
    pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<(TxOutput, usize)> {
        let value = reader.read_u64_le().await?;
        let script_size = VarInt::read(reader).await?;
        let len = 8 + script_size.size() + script_size.value as usize;
        let mut script = vec![0u8; script_size.value as usize];
        let _bytes_read = reader.read_exact(&mut script).await?;
        assert_eq!(_bytes_read, script_size.value as usize);
        return Ok((TxOutput {
            value,
            raw_script: script,
        }, len));
    }

    pub async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u64_le(self.value).await?;
        VarInt::new(self.raw_script.len() as u64).write(writer).await?;
        writer.write_all(&self.raw_script).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use crate::base::Hash;
    use super::*;


    /// Read a transaction from a byte array and check it
    #[tokio::test]
    async fn tx_read() {
        let (tx_bin, tx_hash) = get_tx1();
        let mut cursor = Cursor::new(&tx_bin);
        let tx = Tx::read(&mut cursor).await.unwrap();
        assert_eq!(tx.size, 211);
        assert_eq!(tx.version, 1);
        assert_eq!(tx.hash().await, tx_hash);
    }

    /// If the binary is incomplete, we should get an error
    #[tokio::test]
    async fn read_short() {
        let (tx_bin, tx_hash) = get_tx1();
        let mut cursor = Cursor::new(&tx_bin[0..200]);
        assert!(Tx::read(&mut cursor).await.is_err());
    }

    /// If we supply too many bytes, then the read should succeed and we should have some bytes left over.
    #[tokio::test]
    async fn tx_lomg() {
        let (mut tx_bin, tx_hash) = get_tx1();
        tx_bin.append(&mut vec![0u8; 100]);
        let mut cursor = Cursor::new(&tx_bin[0..300]);
        let tx = Tx::read(&mut cursor).await.unwrap();
        assert_eq!(cursor.position(), 211);
        assert_eq!(tx.size, 211);
        assert_eq!(tx.version, 1);
        assert_eq!(tx.hash().await, tx_hash);
    }

    fn get_tx1() -> (Vec<u8>, Hash) {
        let tx_hex = "01000000018a052edc7ae2136bfc0a860cdc91185ab0d7329107802f0a9c1cd0026c815f75010000006b483045022100e587ef1b4497a6694cad646cab468b6ece2fa98c7f49f9488611ca34eecebd1002205c4ea9066484bd1bffb7fdd7d84b5ae0ee6b7cdc20a8a513e41e420e0633b98841210262142850483b6728b8ecd299e4d0c8cf30ea0636f66205166814e52d73b64b4bffffffff0200000000000000000a006a075354554b2e434fb8ce3f01000000001976a91454cba8da8701174e34aac2bb31d42a88e2c302d088ac00000000";
        let tx_hash = "3abc31f8ff40ffb66d9037e156842fe782e6fa1ae728759263471c68660095f1";
        let tx_bin = hex::decode(tx_hex).unwrap();
        return (tx_bin, Hash::decode(tx_hash).unwrap());
    }
}
