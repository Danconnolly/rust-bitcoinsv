
## Dev Notes
Some notes that I'll need to put into a dev document at some point:

1. I use serde for JSON serialization and deserialization.
2. I use custom de/serialization traits for encoding to Bitcoin Binary standard.


## P2P Notes
1. This library ignores the checksum in the message header. We produce it, because we have to, but we don't check it.
Checking it would mean we would have to read the entire message into memory in one go, which contradicts our streaming
design. Maybe I'll work out a way to calculate the checksum as we stream the message, but for now, we just ignore it.


