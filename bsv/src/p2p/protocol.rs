use crate::p2p::{
    Message, MessageHeader, NetworkAddress, Services, VersionMessage, PROTOCOL_VERSION,
};
use crate::{Error, Result};
use bytes::{Buf, BufMut, BytesMut};
use std::io::{Read, Write};
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

/// P2P protocol handler
pub struct ProtocolHandler {
    magic: u32,
    version: u32,
    services: Services,
    user_agent: String,
    start_height: u32,
    relay: bool,
}

impl ProtocolHandler {
    /// Create a new protocol handler
    pub fn new(magic: u32) -> Self {
        Self {
            magic,
            version: PROTOCOL_VERSION,
            services: Services::NETWORK,
            user_agent: "/rust-bitcoinsv:0.1.0/".to_string(),
            start_height: 0,
            relay: true,
        }
    }

    /// Create a version message
    pub fn create_version_message(&self, peer_addr: IpAddr, peer_port: u16) -> VersionMessage {
        VersionMessage {
            version: self.version,
            services: self.services,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            recv_addr: NetworkAddress {
                timestamp: None,
                services: Services::NONE,
                addr: peer_addr,
                port: peer_port,
            },
            from_addr: NetworkAddress {
                timestamp: None,
                services: self.services,
                addr: IpAddr::V4([127, 0, 0, 1].into()),
                port: 8333,
            },
            nonce: rand::random(),
            user_agent: self.user_agent.clone(),
            start_height: self.start_height,
            relay: self.relay,
        }
    }

    /// Write a message to a writer
    pub fn write_message<W: Write>(&self, writer: &mut W, message: &Message) -> Result<()> {
        // Encode payload
        let mut payload = BytesMut::new();
        message.encode_payload(&mut payload)?;

        // Create header
        let header = MessageHeader::new(self.magic, message.command(), &payload);

        // Write header
        let mut header_buf = BytesMut::with_capacity(MessageHeader::SIZE);
        header.encode(&mut header_buf);
        writer
            .write_all(&header_buf)
            .map_err(|e| Error::Internal(format!("Write error: {}", e)))?;

        // Write payload
        writer
            .write_all(&payload)
            .map_err(|e| Error::Internal(format!("Write error: {}", e)))?;

        Ok(())
    }

    /// Read a message header from a reader
    pub fn read_header<R: Read>(&self, reader: &mut R) -> Result<MessageHeader> {
        let mut header_buf = [0u8; MessageHeader::SIZE];
        reader
            .read_exact(&mut header_buf)
            .map_err(|e| Error::Internal(format!("Read error: {}", e)))?;

        let mut buf = &header_buf[..];
        let header = MessageHeader::decode(&mut buf)?;

        // Verify magic
        if header.magic != self.magic {
            return Err(Error::BadData(format!(
                "Invalid magic: expected {:#x}, got {:#x}",
                self.magic, header.magic
            )));
        }

        Ok(header)
    }

    /// Read a message payload
    pub fn read_payload<R: Read>(&self, reader: &mut R, header: &MessageHeader) -> Result<Vec<u8>> {
        let mut payload = vec![0u8; header.payload_size as usize];
        reader
            .read_exact(&mut payload)
            .map_err(|e| Error::Internal(format!("Read error: {}", e)))?;

        // Note: As per dev.md, we ignore the checksum for streaming design
        // In production, you might want to verify it

        Ok(payload)
    }
}

/// Message framing for reading/writing complete messages
pub struct MessageFramer {
    read_buffer: BytesMut,
    write_buffer: BytesMut,
}

impl MessageFramer {
    pub fn new() -> Self {
        Self {
            read_buffer: BytesMut::with_capacity(1024),
            write_buffer: BytesMut::with_capacity(1024),
        }
    }

    /// Frame a message for sending
    pub fn frame_message(&mut self, magic: u32, message: &Message) -> Result<&[u8]> {
        self.write_buffer.clear();

        // Encode payload first
        let mut payload = BytesMut::new();
        message.encode_payload(&mut payload)?;

        // Create and encode header
        let header = MessageHeader::new(magic, message.command(), &payload);
        header.encode(&mut self.write_buffer);

        // Append payload
        self.write_buffer.put_slice(&payload);

        Ok(&self.write_buffer)
    }

    /// Try to decode a message from the buffer
    pub fn decode_message(&mut self) -> Result<Option<(MessageHeader, Vec<u8>)>> {
        // Need at least a header
        if self.read_buffer.len() < MessageHeader::SIZE {
            return Ok(None);
        }

        // Peek at the header to get payload size
        let mut peek_buf = self.read_buffer.clone();
        let header = MessageHeader::decode(&mut peek_buf)?;

        // Check if we have the complete message
        let total_size = MessageHeader::SIZE + header.payload_size as usize;
        if self.read_buffer.len() < total_size {
            return Ok(None);
        }

        // We have a complete message, consume it
        self.read_buffer.advance(MessageHeader::SIZE);
        let payload = self
            .read_buffer
            .split_to(header.payload_size as usize)
            .to_vec();

        Ok(Some((header, payload)))
    }

    /// Add data to the read buffer
    pub fn add_data(&mut self, data: &[u8]) {
        self.read_buffer.put_slice(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::Hash;
    use crate::p2p::{
        GetBlocksMessage, InvType, Inventory, Message, RejectMessage, MAGIC_MAINNET, MAGIC_TESTNET,
    };
    use hex::FromHex;
    use std::io::Cursor;

    #[test]
    fn test_protocol_handler_version_message() {
        let handler = ProtocolHandler::new(MAGIC_MAINNET);
        let version_msg =
            handler.create_version_message(IpAddr::V4([192, 168, 1, 100].into()), 8333);

        assert_eq!(version_msg.version, PROTOCOL_VERSION);
        assert_eq!(version_msg.services, Services::NETWORK);
        assert_eq!(version_msg.user_agent, "/rust-bitcoinsv:0.1.0/");
        assert_eq!(version_msg.relay, true);

        // Check timestamp is recent
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(version_msg.timestamp >= now - 5);
        assert!(version_msg.timestamp <= now + 5);
    }

    #[test]
    fn test_write_and_read_message() {
        let handler = ProtocolHandler::new(MAGIC_MAINNET);
        let mut buffer = Vec::new();

        // Write a ping message
        let ping_msg = Message::Ping(0x1234567890ABCDEF);
        handler.write_message(&mut buffer, &ping_msg).unwrap();

        // Read it back
        let mut cursor = Cursor::new(&buffer);
        let header = handler.read_header(&mut cursor).unwrap();
        let payload = handler.read_payload(&mut cursor, &header).unwrap();

        assert_eq!(header.command_string(), "ping");
        assert_eq!(header.payload_size, 8);
        assert_eq!(payload.len(), 8);

        // Verify nonce
        let nonce = u64::from_le_bytes([
            payload[0], payload[1], payload[2], payload[3], payload[4], payload[5], payload[6],
            payload[7],
        ]);
        assert_eq!(nonce, 0x1234567890ABCDEF);
    }

    #[test]
    fn test_message_framer() {
        let mut framer = MessageFramer::new();

        // Frame a message
        let inv_msg = Message::Inv(vec![Inventory {
            inv_type: InvType::Block,
            hash: Hash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
        }]);

        let framed = framer
            .frame_message(MAGIC_MAINNET, &inv_msg)
            .unwrap()
            .to_vec();
        assert!(framed.len() > MessageHeader::SIZE);

        // Add the framed data to read buffer
        framer.add_data(&framed);

        // Decode it
        let (header, payload) = framer.decode_message().unwrap().unwrap();
        assert_eq!(header.command_string(), "inv");
        assert_eq!(payload.len(), header.payload_size as usize);
    }

    #[test]
    fn test_message_framer_partial_data() {
        let mut framer = MessageFramer::new();

        // Frame a message with payload
        let msg = Message::Ping(12345);
        let framed = framer.frame_message(MAGIC_MAINNET, &msg).unwrap().to_vec();

        // Add only part of the header
        framer.add_data(&framed[..10]);
        assert!(framer.decode_message().unwrap().is_none());

        // Add rest of header but not payload
        framer.add_data(&framed[10..MessageHeader::SIZE]);
        assert!(framer.decode_message().unwrap().is_none());

        // Add payload
        framer.add_data(&framed[MessageHeader::SIZE..]);
        let result = framer.decode_message().unwrap();
        assert!(result.is_some());

        let (header, payload) = result.unwrap();
        assert_eq!(header.command_string(), "ping");
        assert_eq!(payload.len(), 8);
    }

    #[test]
    fn test_multiple_messages_in_buffer() {
        let mut framer = MessageFramer::new();

        // Frame multiple messages
        let msg1 = Message::Verack;
        let framed1 = framer.frame_message(MAGIC_MAINNET, &msg1).unwrap().to_vec();

        let msg2 = Message::SendHeaders;
        let framed2 = framer.frame_message(MAGIC_MAINNET, &msg2).unwrap().to_vec();

        let msg3 = Message::Ping(42);
        let framed3 = framer.frame_message(MAGIC_MAINNET, &msg3).unwrap().to_vec();

        // Add all at once
        framer.add_data(&framed1);
        framer.add_data(&framed2);
        framer.add_data(&framed3);

        // Decode all messages
        let (header1, _) = framer.decode_message().unwrap().unwrap();
        assert_eq!(header1.command_string(), "verack");

        let (header2, _) = framer.decode_message().unwrap().unwrap();
        assert_eq!(header2.command_string(), "sendheaders");

        let (header3, payload3) = framer.decode_message().unwrap().unwrap();
        assert_eq!(header3.command_string(), "ping");
        assert_eq!(payload3.len(), 8);

        // No more messages
        assert!(framer.decode_message().unwrap().is_none());
    }

    #[test]
    fn test_invalid_magic() {
        let handler = ProtocolHandler::new(MAGIC_MAINNET);
        let wrong_magic_handler = ProtocolHandler::new(MAGIC_TESTNET);

        let mut buffer = Vec::new();

        // Write with testnet magic
        let msg = Message::Verack;
        wrong_magic_handler
            .write_message(&mut buffer, &msg)
            .unwrap();

        // Try to read with mainnet magic
        let mut cursor = Cursor::new(&buffer);
        let result = handler.read_header(&mut cursor);

        assert!(result.is_err());
        match result {
            Err(Error::BadData(msg)) => assert!(msg.contains("Invalid magic")),
            _ => panic!("Expected parse error for invalid magic"),
        }
    }

    #[test]
    fn test_complex_message_handling() {
        let handler = ProtocolHandler::new(MAGIC_MAINNET);
        let mut buffer = Vec::new();

        // Test various message types
        let messages = vec![
            Message::Version(
                handler.create_version_message(IpAddr::V4([10, 0, 0, 1].into()), 8333),
            ),
            Message::Addr(vec![NetworkAddress {
                timestamp: Some(1234567890),
                services: Services::NETWORK,
                addr: IpAddr::V4([192, 168, 1, 1].into()),
                port: 8333,
            }]),
            Message::GetBlocks(GetBlocksMessage {
                version: PROTOCOL_VERSION,
                locator_hashes: vec![Hash::from_hex(
                    "0000000000000000000000000000000000000000000000000000000000000001",
                )
                .unwrap()],
                hash_stop: Hash::from_hex(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                )
                .unwrap(),
            }),
            Message::Reject(RejectMessage {
                message: "tx".to_string(),
                code: 0x10,
                reason: "bad-txns".to_string(),
                data: None,
            }),
        ];

        // Write all messages
        for msg in &messages {
            handler.write_message(&mut buffer, msg).unwrap();
        }

        // Read them back
        let mut cursor = Cursor::new(&buffer);
        let mut read_count = 0;

        while cursor.position() < buffer.len() as u64 {
            let header = handler.read_header(&mut cursor).unwrap();
            let _payload = handler.read_payload(&mut cursor, &header).unwrap();
            read_count += 1;

            // Verify we got the expected command
            match read_count {
                1 => assert_eq!(header.command_string(), "version"),
                2 => assert_eq!(header.command_string(), "addr"),
                3 => assert_eq!(header.command_string(), "getblocks"),
                4 => assert_eq!(header.command_string(), "reject"),
                _ => panic!("Unexpected message count"),
            }
        }

        assert_eq!(read_count, messages.len());
    }

    #[test]
    fn test_protocol_handler_customization() {
        let mut handler = ProtocolHandler::new(MAGIC_MAINNET);
        handler.services = Services(Services::NETWORK.0 | Services::WITNESS.0);
        handler.user_agent = "/custom-client:1.0/".to_string();
        handler.relay = false;
        handler.start_height = 750000;

        let version_msg = handler.create_version_message(IpAddr::V4([127, 0, 0, 1].into()), 8333);

        assert_eq!(version_msg.services.0, 9); // NETWORK | WITNESS
        assert_eq!(version_msg.user_agent, "/custom-client:1.0/");
        assert_eq!(version_msg.relay, false);
        assert_eq!(version_msg.start_height, 750000);
    }
}
