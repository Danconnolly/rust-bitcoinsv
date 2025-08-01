use crate::bitcoin::script::byte_seq::ByteSequence;
use crate::bitcoin::script::Operation;
use crate::bitcoin::{varint_decode, varint_encode, varint_size, Encodable};
use crate::{Error, Result};
use bytes::{Buf, BufMut, Bytes};
use hex::FromHex;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

/// Bitcoin Scripts are used to lock and unlock outputs.
///
/// This struct is a Script in its encoded form and is read-only.
#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Script {
    /// The raw bytes of the script, not including the length of the script.
    pub raw: Bytes,
}

impl Script {
    /// Get the length of the script in bytes
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Check if the script is empty
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// Get the operations in the script (without trailing data)
    pub fn operations(&self) -> Result<Vec<Operation>> {
        let (ops, _) = self.decode()?;
        Ok(ops)
    }

    /// Decode the script, producing a vector of operations and possibly a byte sequence of trailing data.
    pub fn decode(&self) -> Result<(Vec<Operation>, Option<ByteSequence>)> {
        use Operation::*;

        let mut result = Vec::new();
        let mut buf = self.raw.clone();
        let mut trailing = None;
        let mut if_depth = 0;
        while buf.has_remaining() {
            let o = Operation::from_binary(&mut buf)?;
            match o {
                OP_IF | OP_NOTIF => {
                    if_depth += 1;
                }
                OP_ENDIF => {
                    if if_depth > 0 {
                        if_depth -= 1;
                    }
                }
                OP_RETURN => {
                    if if_depth == 0 {
                        trailing = Some(ByteSequence::new(buf.copy_to_bytes(buf.remaining())));
                    }
                }
                _ => {}
            }
            result.push(o);
        }
        Ok((result, trailing))
    }
}

impl Encodable for Script {
    fn from_binary(buffer: &mut dyn Buf) -> Result<Self>
    where
        Self: Sized,
    {
        let sz = varint_decode(buffer)?;
        if buffer.remaining() < sz as usize {
            Err(Error::DataTooSmall)
        } else {
            let raw = buffer.copy_to_bytes(sz as usize);
            Ok(Script { raw })
        }
    }

    fn to_binary(&self, buffer: &mut dyn BufMut) -> Result<()> {
        varint_encode(buffer, self.raw.len() as u64)?;
        buffer.put_slice(&self.raw);
        Ok(())
    }

    fn encoded_size(&self) -> u64 {
        let l = self.raw.len() as u64;
        l + varint_size(l)
    }
}

impl From<Vec<u8>> for Script {
    fn from(value: Vec<u8>) -> Self {
        Self {
            raw: Bytes::from(value),
        }
    }
}

impl FromHex for Script {
    type Error = Error;

    /// Hex encoding is not prefixed by the length.
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> std::result::Result<Self, Self::Error> {
        let raw = hex::decode(hex)?;
        Ok(Self {
            raw: Bytes::from(raw),
        })
    }
}

impl Debug for Script {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Script({})", hex::encode(&self.raw))
    }
}

#[cfg(test)]
mod tests {
    use crate::bitcoin::{Encodable, Script};
    use hex::FromHex;

    /// Test reading a script from hex.
    #[test]
    fn script_read_hex() {
        // this script comes from input 0 from tx 60dcda63c57420077d67e3ae6684a1654cf9f9cc1b8edd569a847f2b5109b739
        let s = Script::from_hex("47304402207df65c96172de240e6232daeeeccccf8655cb4aba38d968f784e34c6cc047cd30220078216eefaddb915ce55170348c3363d013693c543517ad59188901a0e7f8e50412103be56e90fb443f554140e8d260d7214c3b330cfb7da83b3dd5624f85578497841").expect("Failed to parse script from hex for test");
        assert_eq!(107, s.encoded_size()); // 106 bytes + 1 for size as varint
    }

    /// Test decoding a script.
    #[test]
    fn test_decode() {
        // this script comes from input 0 from tx 60dcda63c57420077d67e3ae6684a1654cf9f9cc1b8edd569a847f2b5109b739
        let s = Script::from_hex("47304402207df65c96172de240e6232daeeeccccf8655cb4aba38d968f784e34c6cc047cd30220078216eefaddb915ce55170348c3363d013693c543517ad59188901a0e7f8e50412103be56e90fb443f554140e8d260d7214c3b330cfb7da83b3dd5624f85578497841").expect("Failed to parse script from hex for decode test");
        let (ops, trailing) = s.decode().expect("Failed to decode script for test");
        assert_eq!(2, ops.len());
        assert!(trailing.is_none());
    }

    /// Test with an op_return
    #[test]
    fn test_opreturn_decode() {
        // this script comes from output 0 from tx 6920f4ec65cea88052c94b1f114c4b038a52af42b1b5c3cb4acba3b4e9bec743
        let s = Script::from_hex("006a20d9d22fff84fbf87e2bb5d3fe2d537e68436a8bec83df40a2e3ff705c0b8e0d1b10a67f8a1e943f47c69ad57c6750768c43").expect("Failed to parse OP_RETURN script from hex for test");
        let (ops, trailing) = s.decode().expect("Failed to decode script for test");
        assert_eq!(2, ops.len());
        assert!(trailing.is_some());
        assert_eq!(trailing.expect("Should have trailing data").len(), 50);
    }
}
