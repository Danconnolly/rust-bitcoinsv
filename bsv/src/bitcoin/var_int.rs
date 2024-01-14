use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::bitcoin::binary::Encodable;

/// The VarInt Bitcoin data type with async serialization.
// Code based on `<https://github.com/brentongunning/rust-sv>`
// Improvement: implement a new function sizeof() which calculates the size without instantiating a VarInt object.
//    * Also implement a function to just return the raw encoded form. Consider removing this as a struct, it doesn't
//      seem very useful.
pub struct VarInt {
    pub value: u64,
    pub raw: Vec<u8>,
}

impl VarInt {
    pub fn new(v: u64) -> VarInt {
        VarInt {
            value: v,
            raw: VarInt::raw_from_v(v),
        }
    }

    fn raw_from_v(v: u64) -> Vec<u8> {
        match v {
            0..=252 => vec![v as u8],
            253..=0xffff => {
                let mut o = vec![0xfd];
                o.extend_from_slice(&(v as u16).to_le_bytes());
                o
            }
            0x10000..=0xffffffff => {
                let mut o = vec![0xfe];
                o.extend_from_slice(&(v as u32).to_le_bytes());
                o
            }
            _ => {
                let mut o = vec![0xff];
                o.extend_from_slice(&(v as u64).to_le_bytes());
                o
            }
        }
    }
}

impl Encodable for VarInt {
    fn read<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<VarInt> {
        let n0 = reader.read_u8().unwrap();
        let v = match n0 {
            0xff => reader.read_u64::<LittleEndian>().unwrap(),
            0xfe => reader.read_u32::<LittleEndian>().unwrap() as u64,
            0xfd => reader.read_u16::<LittleEndian>().unwrap() as u64,
            _ => n0 as u64 };
        Ok( VarInt {
            value: v,
            raw: VarInt::raw_from_v(v),
        })
    }

    fn write<R: WriteBytesExt + Send>(&self, writer: &mut R) -> crate::Result<()> {
        writer.write_all(&self.raw).unwrap();
        Ok(())
    }

    fn size(&self) -> usize {
        self.raw.len()
    }
}


#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::VarInt;
    use crate::bitcoin::binary::Encodable;

    #[test]
    fn size() {
        assert_eq!(VarInt::new(0).size(), 1);
        assert_eq!(VarInt::new(253).size(), 3);
        assert_eq!(VarInt::new(u16::max_value() as u64).size(), 3);
        assert_eq!(VarInt::new(u32::max_value() as u64).size(), 5);
        assert_eq!(VarInt::new(u64::max_value()).size(), 9);
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
        let vi = VarInt::new(n);
        let mut v = Vec::new();
        vi.write(&mut v).unwrap();
        assert_eq!(VarInt::read(&mut Cursor::new(&v)).unwrap().value, n);
    }

    #[test]
    fn test_known_values() {
        let mut v = Vec::new();
        VarInt::new(0).write(&mut v).unwrap();
        assert_eq!(v, vec![0]);
        v.clear();
        VarInt::new(1).write(&mut v).unwrap();
        assert_eq!(v, vec![1]);
        v.clear();
        VarInt::new(252).write(&mut v).unwrap();
        assert_eq!(v, vec![252]);
        v.clear();
        VarInt::new(253).write(&mut v).unwrap();
        assert_eq!(v, vec![253, 253, 0]);
        v.clear();
        VarInt::new(254).write(&mut v).unwrap();
        assert_eq!(v, vec![253, 254, 0]);
        v.clear();
        VarInt::new(255).write(&mut v).unwrap();
        assert_eq!(v, vec![253, 255, 0]);
        v.clear();
        VarInt::new(256).write(&mut v).unwrap();
        assert_eq!(v, vec![253, 0, 1]);
        v.clear();
        VarInt::new(65535).write(&mut v).unwrap();
        assert_eq!(v, vec![253, 255, 255]);
        v.clear();
        VarInt::new(65536).write(&mut v).unwrap();
        assert_eq!(v, vec![254, 0, 0, 1, 0]);
        v.clear();
        VarInt::new(65537).write(&mut v).unwrap();
        assert_eq!(v, vec![254, 1, 0, 1, 0]);
        v.clear();
        VarInt::new(4294967295).write(&mut v).unwrap();
        assert_eq!(v, vec![254, 255, 255, 255, 255]);
        v.clear();
        VarInt::new(4294967296).write(&mut v).unwrap();
        assert_eq!(v, vec![255, 0, 0, 0, 0, 1, 0, 0, 0]);
        v.clear();
        VarInt::new(4294967297).write(&mut v).unwrap();
        assert_eq!(v, vec![255, 1, 0, 0, 0, 1, 0, 0, 0]);
    }
}