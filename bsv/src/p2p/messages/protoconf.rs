use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use log::warn;
use crate::bitcoin::{varint_decode, Encodable, varint_size, varint_encode};

/// Protocol configuration message.
///
/// The message enables the sender to advertise various connection parameters to a remote peer.
/// Note that according to the spec the maximum size of this message is 1_048_576 bytes and the
/// fact that the specification specifies this implies that this is true even if the protoconf
/// message is not explicitly supported, which is kind of strange.
///
/// Note that the protoconf does not require an acknoledgement. Which means it is a statement of intent
/// and the peer that sent the message must respect it.
///
/// Specification: https://github.com/bitcoin-sv-specs/protocol/blob/master/p2p/protoconf.md
// IMPROVEMENT: the strange size exception could be a mis-interpretation of the spec. Check the
//              SV Node code to see if it has the same exception. Note the exception is also
//              encoded in the message header validation code.
// IMPROVEMENT: should we make this more resilient? able to handle only one field? how many fields does a 70015 node send?
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Protoconf {
    /// Max Receive Payload Length.
    ///
    /// This is the maximum length in bytes of a message that the sender is willing to accept.
    /// This setting also removes the limit on the number of inventory vectors that can be sent in a
    /// single message, the limit is instead determined by the maximum message size.
    pub max_recv_payload_length: u32,
    /// Stream Policies
    pub stream_policies: String,
}

impl Protoconf {
    /// Creates a new `Protoconf` message.
    pub fn new(max_recv_payload_length: u32) -> Protoconf {
        Protoconf {
            max_recv_payload_length,
            stream_policies: String::from("Default"),
        }
    }
}

impl Default for Protoconf {
    fn default() -> Protoconf {
        Protoconf {
            max_recv_payload_length: 1_048_576,
            stream_policies: String::from("Default"),
        }
    }
}

impl Encodable for Protoconf {
    fn decode<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let num_entries = varint_decode(reader)?;
        if num_entries < 2 {
            return Err(crate::Error::BadData("Protoconf must have at least 2 entries".to_string()));
        } else if num_entries > 2 {
            warn!("Protoconf has more than 2 entries, ignoring extra entries.");
        }
        let max_recv_payload_length = reader.read_u32::<LittleEndian>()?;
        let string_size = varint_decode(reader)?;
        let mut string_bytes = vec![0; string_size as usize];
        reader.read_exact(&mut string_bytes)?;
        let stream_policies = String::from_utf8(string_bytes)?;
        Ok(Protoconf {
            max_recv_payload_length,
            stream_policies,
        })
    }

    fn encode_into<W: WriteBytesExt + Send>(&self, writer: &mut W) -> crate::Result<()> {
        varint_encode(writer, 2)?;
        writer.write_u32::<LittleEndian>(self.max_recv_payload_length)?;
        varint_encode(writer, self.stream_policies.len() as u64)?;
        writer.write_all(self.stream_policies.as_bytes())?;
        Ok(())
    }

    fn size(&self) -> usize {
        varint_size(2) + 4 + varint_size(self.stream_policies.len() as u64) + self.stream_policies.len()
    }
}
