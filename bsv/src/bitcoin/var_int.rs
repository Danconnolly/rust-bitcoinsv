use bytes::{Buf, BufMut};

/// The minimum size (in bytes) of an encoded VarInt.
pub const VARINT_MIN_SIZE: usize = 1;
/// The maximum size (in bytes) of an encoded VarInt.
pub const VARINT_MAX_SIZE: usize = 9;

/// The size of the value encoded as a varint.
pub fn varint_size(value: u64) -> u64 {
    match value {
        0..=252 => 1,
        253..=0xffff => 3,
        0x10000..=0xffffffff => 5,
        _ => 9,
    }
}

/// Read a varint from the buffer.
pub fn varint_decode(buffer: &mut dyn Buf) -> crate::Result<u64> {
    let n0 = buffer.get_u8();
    let v = match n0 {
        0xff => buffer.get_u64_le(),
        0xfe => buffer.get_u32_le() as u64,
        0xfd => buffer.get_u16_le() as u64,
        _ => n0 as u64,
    };
    Ok(v)
}

/// Write a varint to the buffer.
pub fn varint_encode(buffer: &mut dyn BufMut, value: u64) -> crate::Result<()> {
    match value {
        0..=252 => buffer.put_u8(value as u8),
        253..=0xffff => {
            buffer.put_u8(0xfd);
            buffer.put_u16_le(value as u16);
        }
        0x10000..=0xffffffff => {
            buffer.put_u8(0xfe);
            buffer.put_u32_le(value as u32);
        }
        _ => {
            buffer.put_u8(0xff);
            buffer.put_u64_le(value as u64);
        }
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn size() {
        assert_eq!(varint_size(0), 1);
        assert_eq!(varint_size(253), 3);
        assert_eq!(varint_size(u16::max_value() as u64), 3);
        assert_eq!(varint_size(u32::max_value() as u64), 5);
        assert_eq!(varint_size(u64::max_value()), 9);
    }

    #[test]
    fn write_read() {
        write_read_value(0);
        write_read_value(253);
        write_read_value(u16::max_value() as u64);
        write_read_value(u32::max_value() as u64);
        write_read_value(u64::max_value());
    }

    fn write_read_value(n: u64) {
        let mut v = BytesMut::new();
        varint_encode(&mut v, n).expect("Failed to encode varint");
        let j = varint_decode(&mut v).expect("Failed to decode varint");
        assert_eq!(j, n);
    }

    #[test]
    fn test_known_values() {
        let mut v = Vec::new();
        varint_encode(&mut v, 0).expect("Failed to encode varint");
        assert_eq!(v, vec![0]);
        v.clear();
        varint_encode(&mut v, 1).expect("Failed to encode varint");
        assert_eq!(v, vec![1]);
        v.clear();
        varint_encode(&mut v, 252).expect("Failed to encode varint");
        assert_eq!(v, vec![252]);
        v.clear();
        varint_encode(&mut v, 253).expect("Failed to encode varint");
        assert_eq!(v, vec![253, 253, 0]);
        v.clear();
        varint_encode(&mut v, 254).expect("Failed to encode varint");
        assert_eq!(v, vec![253, 254, 0]);
        v.clear();
        varint_encode(&mut v, 255).expect("Failed to encode varint");
        assert_eq!(v, vec![253, 255, 0]);
        v.clear();
        varint_encode(&mut v, 256).expect("Failed to encode varint");
        assert_eq!(v, vec![253, 0, 1]);
        v.clear();
        varint_encode(&mut v, 65535).expect("Failed to encode varint");
        assert_eq!(v, vec![253, 255, 255]);
        v.clear();
        varint_encode(&mut v, 65536).expect("Failed to encode varint");
        assert_eq!(v, vec![254, 0, 0, 1, 0]);
        v.clear();
        varint_encode(&mut v, 65537).expect("Failed to encode varint");
        assert_eq!(v, vec![254, 1, 0, 1, 0]);
        v.clear();
        varint_encode(&mut v, 4294967295).expect("Failed to encode varint");
        assert_eq!(v, vec![254, 255, 255, 255, 255]);
        v.clear();
        varint_encode(&mut v, 4294967296).expect("Failed to encode varint");
        assert_eq!(v, vec![255, 0, 0, 0, 0, 1, 0, 0, 0]);
        v.clear();
        varint_encode(&mut v, 4294967297).expect("Failed to encode varint");
        assert_eq!(v, vec![255, 1, 0, 0, 0, 1, 0, 0, 0]);
    }
}
