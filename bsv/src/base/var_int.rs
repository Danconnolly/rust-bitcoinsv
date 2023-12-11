use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The VarInt Bitcoin data type with async serialization.
/// Code based on https://github.com/brentongunning/rust-sv
pub struct VarInt {
    pub value: u64
}

impl VarInt {
    pub fn new(v: u64) -> VarInt {
        VarInt {
            value: v
        }
    }

    pub fn size(&self) -> usize {
        return if self.value <= 252 {
            1
        } else if self.value <= 0xffff {
            3
        } else if self.value <= 0xffffffff {
            5
        } else {
            9
        };
    }

    ///
    /// Writes the var int to bytes
    pub async fn write<R: AsyncWrite + Unpin>(&self, writer: &mut R) -> io::Result<()> {
        if self.value <= 252 {
            writer.write_u8(self.value as u8).await.unwrap();
        } else if self.value <= 0xffff {
            writer.write_u8(0xfd).await.unwrap();
            writer.write_u16_le(self.value as u16).await.unwrap();
        } else if self.value <= 0xffffffff {
            writer.write_u8(0xfe).await.unwrap();
            writer.write_u32_le(self.value as u32).await.unwrap();
        } else {
            writer.write_u8(0xff).await.unwrap();
            writer.write_u64_le(self.value).await.unwrap();
        }
        Ok(())
    }

    /// Reads a var int from bytes
    pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<VarInt> {
        let n0 = reader.read_u8().await.unwrap();
        Ok( VarInt {
            value: match n0 {
            0xff => reader.read_u64_le().await.unwrap(),
            0xfe => reader.read_u32_le().await.unwrap() as u64,
            0xfd => reader.read_u16_le().await.unwrap() as u64,
            _ => n0 as u64,
        }})
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::VarInt;

    #[test]
    fn size() {
        assert_eq!(VarInt::new(0).size(), 1);
        assert_eq!(VarInt::new(253).size(), 3);
        assert_eq!(VarInt::new(u16::max_value() as u64).size(), 3);
        assert_eq!(VarInt::new(u32::max_value() as u64).size(), 5);
        assert_eq!(VarInt::new(u64::max_value()).size(), 9);
    }

    #[tokio::test]
    async fn write_read() {
        write_read_value(0).await;
        write_read_value(253).await;
        write_read_value(u16::max_value() as u64).await;
        write_read_value(u32::max_value() as u64).await;
        write_read_value(u64::max_value()).await;
    }

    async fn write_read_value(n: u64) {
        let vi = VarInt::new(n);
        let mut v = Vec::new();
        vi.write(&mut v).await.unwrap();
        assert_eq!(VarInt::read(&mut Cursor::new(&v)).await.unwrap().value, n);
    }

    #[tokio::test]
    async fn test_known_values() {
        let mut v = Vec::new();
        VarInt::new(0).write(&mut v).await.unwrap();
        assert_eq!(v, vec![0]);
        v.clear();
        VarInt::new(1).write(&mut v).await.unwrap();
        assert_eq!(v, vec![1]);
        v.clear();
        VarInt::new(252).write(&mut v).await.unwrap();
        assert_eq!(v, vec![252]);
        v.clear();
        VarInt::new(253).write(&mut v).await.unwrap();
        assert_eq!(v, vec![253, 253, 0]);
        v.clear();
        VarInt::new(254).write(&mut v).await.unwrap();
        assert_eq!(v, vec![253, 254, 0]);
        v.clear();
        VarInt::new(255).write(&mut v).await.unwrap();
        assert_eq!(v, vec![253, 255, 0]);
        v.clear();
        VarInt::new(256).write(&mut v).await.unwrap();
        assert_eq!(v, vec![253, 0, 1]);
        v.clear();
        VarInt::new(65535).write(&mut v).await.unwrap();
        assert_eq!(v, vec![253, 255, 255]);
        v.clear();
        VarInt::new(65536).write(&mut v).await.unwrap();
        assert_eq!(v, vec![254, 0, 0, 1, 0]);
        v.clear();
        VarInt::new(65537).write(&mut v).await.unwrap();
        assert_eq!(v, vec![254, 1, 0, 1, 0]);
        v.clear();
        VarInt::new(4294967295).write(&mut v).await.unwrap();
        assert_eq!(v, vec![254, 255, 255, 255, 255]);
        v.clear();
        VarInt::new(4294967296).write(&mut v).await.unwrap();
        assert_eq!(v, vec![255, 0, 0, 0, 0, 1, 0, 0, 0]);
        v.clear();
        VarInt::new(4294967297).write(&mut v).await.unwrap();
        assert_eq!(v, vec![255, 1, 0, 0, 0, 1, 0, 0, 0]);
    }
}