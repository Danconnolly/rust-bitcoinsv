use async_trait::async_trait;
use hex::FromHex;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::{varint_decode, varint_encode, varint_size, AsyncEncodable, Encodable, TxOutput};
use crate::bitcoin::script::Operation;
use crate::BsvResult;

/// A Script represents a Bitcoin Script.
///
/// Bitcoin Scripts are used to lock outputs and unlock those outputs in inputs.
///
/// This is a very simplified initial implementation that only encodes Script from Hex.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Script {
    pub raw: Vec<u8>,
}

impl Script {
    fn operations(&self) -> BsvResult<Vec<Operation>> {
        let mut result = Vec::new();
        // for i in self.raw {
        //     // result.push()
        // }
        Ok(result)
    }
}

impl FromHex for Script {
    type Error = crate::BsvError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let raw = hex::decode(hex)?;
        Ok(Script { raw })
    }
}

#[async_trait]
impl AsyncEncodable for Script {
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> BsvResult<Self> where Self: Sized {
        todo!()
    }

    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> BsvResult<()> {
        todo!()
    }

    fn async_size(&self) -> usize {
        todo!()
    }
}
