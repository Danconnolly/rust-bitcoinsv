use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};

/// The size of the value encoded as a varint.
pub fn varint_size(value: u64) -> usize {
    match value {
        0..=252 => 1,
        253..=0xffff => 3,
        0x10000..=0xffffffff => 5,
        _ => 9,
    }
}

/// Decode a variable length integer from a byte stream, async version.
pub async fn varint_decode<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<u64> {
    let n0 = reader.read_u8().await.unwrap();
    let v = match n0 {
        0xff => reader.read_u64_le().await.unwrap(),
        0xfe => reader.read_u32_le().await.unwrap() as u64,
        0xfd => reader.read_u16_le().await.unwrap() as u64,
        _ => n0 as u64 };
    Ok(v)
}

/// Encode a variable length integer into a byte stream, async version.
pub async fn varint_encode<W: AsyncWrite + Unpin + Send>(writer: &mut W, value: u64) -> crate::Result<()> {
    match value {
        0..=252 => writer.write_u8(value as u8).await?,
        253..=0xffff => {
            writer.write_u8(0xfd).await?;
            writer.write_u16_le(value as u16).await?;
        }
        0x10000..=0xffffffff => {
            writer.write_u8(0xfe).await?;
            writer.write_u32_le(value as u32).await?;
        }
        _ => {
            writer.write_u8(0xff).await?;
            writer.write_u64_le(value).await?;
        }
    };
    Ok(())
}


#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;

    #[test]
    fn size() {
        assert_eq!(varint_size(0), 1);
        assert_eq!(varint_size(253), 3);
        assert_eq!(varint_size(u16::max_value() as u64), 3);
        assert_eq!(varint_size(u32::max_value() as u64), 5);
        assert_eq!(varint_size(u64::max_value()), 9);
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
        let mut v: Vec<u8> = Vec::new();
        let _ = varint_encode(&mut v, n).await.unwrap();
        let j = varint_decode(&mut Cursor::new(&v)).await.unwrap();
        assert_eq!(j, n);
    }

    #[tokio::test]
    async fn test_known_values() {
        let mut v = Vec::new();
        let _ = varint_encode(&mut v, 0).await.unwrap();
        assert_eq!(v, vec![0]);
        v.clear();
        let _ = varint_encode(&mut v, 1).await.unwrap();
        assert_eq!(v, vec![1]);
        v.clear();
        let _ = varint_encode(&mut v, 252).await.unwrap();
        assert_eq!(v, vec![252]);
        v.clear();
        let _ = varint_encode(&mut v, 253).await.unwrap();
        assert_eq!(v, vec![253, 253, 0]);
        v.clear();
        let _ = varint_encode(&mut v, 254).await.unwrap();
        assert_eq!(v, vec![253, 254, 0]);
        v.clear();
        let _ = varint_encode(&mut v, 255).await.unwrap();
        assert_eq!(v, vec![253, 255, 0]);
        v.clear();
        let _ = varint_encode(&mut v, 256).await.unwrap();
        assert_eq!(v, vec![253, 0, 1]);
        v.clear();
        let _ = varint_encode(&mut v, 65535).await.unwrap();
        assert_eq!(v, vec![253, 255, 255]);
        v.clear();
        let _ = varint_encode(&mut v, 65536).await.unwrap();
        assert_eq!(v, vec![254, 0, 0, 1, 0]);
        v.clear();
        let _ = varint_encode(&mut v, 65537).await.unwrap();
        assert_eq!(v, vec![254, 1, 0, 1, 0]);
        v.clear();
        let _ = varint_encode(&mut v, 4294967295).await.unwrap();
        assert_eq!(v, vec![254, 255, 255, 255, 255]);
        v.clear();
        let _ = varint_encode(&mut v, 4294967296).await.unwrap();
        assert_eq!(v, vec![255, 0, 0, 0, 0, 1, 0, 0, 0]);
        v.clear();
        let _ = varint_encode(&mut v, 4294967297).await.unwrap();
        assert_eq!(v, vec![255, 1, 0, 0, 0, 1, 0, 0, 0]);
    }
}