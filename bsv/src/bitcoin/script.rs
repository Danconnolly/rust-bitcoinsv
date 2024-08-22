use hex::FromHex;

/// A Script represents a Bitcoin Script.
///
/// Bitcoin Scripts are used to lock outputs and unlock those outputs in inputs.
///
/// This is a very simplified initial implementation that only encodes Script from Hex.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Script {
    pub raw: Vec<u8>,
}

impl FromHex for Script {
    type Error = crate::Error;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let raw = hex::decode(hex)?;
        Ok(Script { raw })
    }
}
