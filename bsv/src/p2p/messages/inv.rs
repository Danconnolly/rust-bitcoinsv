use std::fmt;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::{AsyncEncodable, varint_decode, varint_encode, varint_size, Hash};


/// Inventory payload describing objects a node knows about
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Inv {
    /// List of objects announced
    pub objects: Vec<InvItem>,
}

#[async_trait]
impl AsyncEncodable for Inv {
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::BsvResult<Self> where Self: Sized {
        let num_objects = varint_decode(reader).await? as usize;
        // if num_objects > MAX_INV_ENTRIES {
        //     let msg = format!("Num objects exceeded maximum: {}", num_objects);
        //     return Err(crate::Error::BadData(msg));
        // }
        let mut objects = Vec::with_capacity(num_objects);
        for _ in 0..num_objects {
            objects.push(InvItem::async_from_binary(reader).await?);
        }
        Ok(Inv { objects })
    }

    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::BsvResult<()> {
        // if self.objects.len() as u64 > MAX_INV_ENTRIES {
        //     let msg = format!("Too many objects: {}", self.objects.len());
        //     return Err(crate::Error::BadData(msg));
        // }
        varint_encode(writer, self.objects.len() as u64).await?;
        for object in self.objects.iter() {
            object.async_to_binary(writer).await?;
        }
        Ok(())
    }

    fn async_size(&self) -> usize {
        varint_size(self.objects.len() as u64) + self.objects.len() * InvItem::SIZE
    }
}

impl fmt::Display for Inv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut objects = String::new();
        for object in &self.objects {
            if objects.len() == 0 {
                objects = format!("{}", object);
            } else {
                objects += &*format!(", {}", object);
            }
        }
        write!(f, "Inv(n={}, [{}])", self.objects.len(), objects)
    }
}

/// Inventory item types
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum InvType {
    /// May be ignored
    InvError = 0,
    /// Hash of a transaction
    Tx = 1,
    /// Hash of a block header.
    Block = 2,
    /// Hash of a block header. Indicates the reply should be a cmpctblock message.
    CompactBlock = 4,
}

impl TryFrom<u32> for InvType {
    type Error = crate::BsvError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InvType::InvError),
            1 => Ok(InvType::Tx),
            2 => Ok(InvType::Block),
            4 => Ok(InvType::CompactBlock),
            _ => Err(crate::BsvError::BadData("Invalid inventory type".to_string())),
        }
    }

}


/// An inventory item, one element from a vector of inventory items.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct InvItem {
    /// Type of object
    pub obj_type: InvType,
    /// Hash of the object
    pub hash: Hash,
}

impl InvItem {
    /// Size of the inventory item in bytes
    pub const SIZE: usize = 36;
}

impl fmt::Display for InvType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvType::InvError => write!(f, "Error"),
            InvType::Tx => write!(f, "Tx"),
            InvType::Block => write!(f, "Block"),
            InvType::CompactBlock => write!(f, "CompactBlock"),
        }
    }
}


#[async_trait]
impl AsyncEncodable for InvItem {
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::BsvResult<Self> where Self: Sized {
        let obj_type = reader.read_u32_le().await?;
        let hash = Hash::async_from_binary(reader).await?;
        Ok(InvItem { obj_type: InvType::try_from(obj_type)?, hash })
    }

    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::BsvResult<()> {
        match self.obj_type {
            InvType::InvError => writer.write_u32_le(0).await?,
            InvType::Tx => writer.write_u32_le(1).await?,
            InvType::Block => writer.write_u32_le(2).await?,
            InvType::CompactBlock => writer.write_u32_le(4).await?,
        }
        self.hash.async_to_binary(writer).await
    }

    fn async_size(&self) -> usize {
        InvItem::SIZE
    }
}

impl fmt::Display for InvItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.obj_type, self.hash)
    }
}