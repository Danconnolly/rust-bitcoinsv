use crate::bitcoin::{Encodable, Operation, Script};
use crate::Result;
use bytes::Bytes;
use std::cmp::max;

/// ScriptBuilder can be used to build [Script]s.
///
/// todo: add grammar checker
pub struct ScriptBuilder {
    /// the operations
    ops: Vec<Operation>,
    /// trailing data to be added
    trailing: Option<Bytes>,
}

impl ScriptBuilder {
    /// Create a new Scriptbuilder for constructing a [Script].
    pub fn new() -> ScriptBuilder {
        Self {
            ops: Vec::new(),
            trailing: None,
        }
    }

    /// Build the script.
    pub fn build(&self) -> Result<Script> {
        // 1000 bytes should hold most scripts
        let cap = match self.trailing.clone() {
            None => 1_000,
            Some(v) => max(1_000, v.len()),
        };
        let mut buffer = Vec::with_capacity(cap);
        let mut last_opreturn = false;
        for o in self.ops.iter() {
            o.to_binary(&mut buffer)?;
            if *o == Operation::OP_RETURN {
                last_opreturn = true;
            } else {
                last_opreturn = false;
            }
        }
        if self.trailing.is_some() {
            let o = self.trailing.clone().unwrap();
            if !last_opreturn {
                Operation::OP_RETURN.to_binary(&mut buffer)?;
            }
            buffer.append(&mut o.to_vec());
        }
        Ok(Script {
            raw: Bytes::from(buffer),
        })
    }

    /// Add an operation to the script.
    pub fn add(&mut self, op: Operation) -> &mut ScriptBuilder {
        self.ops.push(op);
        self
    }

    /// Set the trailing bytes (a.k.a OP_RETURN data) for the script.
    ///
    /// Additional data can be appended to the end of the script. This should be preceded by
    /// an OP_RETURN.
    ///
    /// When building the Script, if an OP_RETURN is not present at the end of the script, then one
    /// will be appended.
    pub fn set_trailing(&mut self, trailing: Bytes) -> &mut ScriptBuilder {
        self.trailing = Some(trailing);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::ByteSequence;
    use bytes::Bytes;
    use hex_literal::hex;

    #[test]
    fn create_p2pkh_output_script() {
        // from tx d2bb697e3555cb0e4a82f0d4990d1c826eee9f648a5efc598f648bdb524093ff output 0
        use Operation::*;
        let byteseq = ByteSequence::new(Bytes::from(
            &hex!("6f67988ec4b7bf498c9164d76b52dffdc805ff8c")[..],
        ));
        let script = ScriptBuilder::new()
            .add(OP_DUP)
            .add(OP_HASH160)
            .add(OP_PUSH(byteseq))
            .add(OP_EQUALVERIFY)
            .add(OP_CHECKSIG)
            .build()
            .unwrap();
        assert_eq!(script.raw.len(), 25);
        assert_eq!(
            script.raw,
            Bytes::from(&hex!("76a9146f67988ec4b7bf498c9164d76b52dffdc805ff8c88ac")[..])
        );
    }
}
