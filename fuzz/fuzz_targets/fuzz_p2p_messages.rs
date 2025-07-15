#![no_main]

use libfuzzer_sys::fuzz_target;
use bitcoinsv::p2p::{MessageHeader, NetworkAddress};
use bytes::Buf;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    // Test message header parsing
    if data.len() >= MessageHeader::SIZE {
        let mut cursor = Cursor::new(data);
        if let Ok(header) = MessageHeader::decode(&mut cursor) {
            // Test command string extraction
            let _ = header.command_string();
            
            // Verify payload size limit
            assert!(header.payload_size <= 32 * 1024 * 1024);
        }
    }
    
    // Test network address parsing
    if data.len() >= 26 {
        let mut cursor = Cursor::new(data);
        if let Ok(addr) = NetworkAddress::decode(&mut cursor) {
            // Verify we can encode it back
            use bytes::BytesMut;
            let mut buffer = BytesMut::new();
            addr.encode(&mut buffer);
        }
    }
});