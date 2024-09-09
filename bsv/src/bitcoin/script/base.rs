use async_trait::async_trait;
use bytes::{Bytes, Buf};
use hex::FromHex;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::{varint_decode, varint_encode, varint_size, AsyncEncodable, Encodable};
use crate::bitcoin::script::Operation;
use crate::BsvResult;

/// Bitcoin Scripts are used to lock and unlock outputs.
///
/// This struct is a Script in its encoded form and is read-only. Use [decode()]
/// to examine a script or [ScriptBuilder] to build a script.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Script {
    pub raw: Bytes
}

impl Script {
    pub fn decode(&self) -> BsvResult<Vec<Operation>> {
        let mut result = Vec::new();
        let mut buf = self.raw.clone();
        while buf.has_remaining() {
            let o = Operation::from_binary(&mut buf)?;
            result.push(o);
        }
        Ok(result)
    }
}

impl From<Vec<u8>> for Script {
    fn from(value: Vec<u8>) -> Self {
        Self {
            raw: Bytes::from(value)
        }
    }
}

impl FromHex for Script {
    type Error = crate::BsvError;

    /// Hex encoding is not prefixed by the length.
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let raw= hex::decode(hex)?;
        Ok(Self {
            raw: Bytes::from(raw)
        })
    }
}


#[async_trait]
impl AsyncEncodable for Script {
    /// Decode a Script from an async reader.
    ///
    /// A script is always encoded with its size.
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> BsvResult<Self>
    where
        Self: Sized
    {
        let size = varint_decode(reader).await?;
        // todo: check size is not too big
        let mut buffer = vec![0u8; size as usize];
        let i = reader.read_exact(&mut buffer).await?;
        Ok(Self {
          raw: Bytes::from(buffer),
        })
    }

    /// Encode a Script from to an async writer.
    ///
    /// A script is always encoded with its size.
    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> BsvResult<()> {
        varint_encode(writer, self.raw.len() as u64).await?;
        writer.write_all(&self.raw).await?;
        Ok(())
    }

    /// Get the size of the encoded script.
    ///
    /// The size is the number of bytes plus the number of bytes needed to encode its size.
    fn async_size(&self) -> usize {
        let l = self.raw.len();
        varint_size(l as u64) + l
    }
}

/// A ScriptBuilder can be used to build a [Script].
pub struct ScriptBuilder {
    // todo: implement
}

#[cfg(test)]
mod tests {
    use hex::FromHex;
    use crate::bitcoin::{AsyncEncodable, Script};

    /// Test reading a script from hex.
    #[test]
    fn script_read_hex() {
        // this script comes from input 0 from tx 60dcda63c57420077d67e3ae6684a1654cf9f9cc1b8edd569a847f2b5109b739
        let s = Script::from_hex("47304402207df65c96172de240e6232daeeeccccf8655cb4aba38d968f784e34c6cc047cd30220078216eefaddb915ce55170348c3363d013693c543517ad59188901a0e7f8e50412103be56e90fb443f554140e8d260d7214c3b330cfb7da83b3dd5624f85578497841").unwrap();
        assert_eq!(107, s.async_size());        // 106 bytes + 1 for size as varint
    }

    /// Test decoding a script.
    #[test]
    fn test_decode() {
        // this script comes from input 0 from tx 60dcda63c57420077d67e3ae6684a1654cf9f9cc1b8edd569a847f2b5109b739
        let s = Script::from_hex("47304402207df65c96172de240e6232daeeeccccf8655cb4aba38d968f784e34c6cc047cd30220078216eefaddb915ce55170348c3363d013693c543517ad59188901a0e7f8e50412103be56e90fb443f554140e8d260d7214c3b330cfb7da83b3dd5624f85578497841").unwrap();
        let ops = s.decode().unwrap();
        assert_eq!(2, ops.len());
    }
}