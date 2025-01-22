use crate::bitcoin::{Encodable, Operation, PrivateKey, Script};
use crate::Result;
use bytes::{Buf, BufMut, Bytes};
use std::cmp::max;

/// A ScriptToken represents an element in a script. In the simplist case it is a single
/// operation, but it can represent a signature that can only be calculated when the transaction
/// is built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptToken {
    /// An Operation.
    Op(Operation),
    /// A signature that can be verified using OP_CHECKSIG.
    CheckSigSignature(PrivateKey),
}

impl Encodable for ScriptToken {
    fn from_binary(buffer: &mut dyn Buf) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(ScriptToken::Op(Operation::from_binary(buffer)?))
    }

    fn to_binary(&self, buffer: &mut dyn BufMut) -> Result<()> {
        match self {
            ScriptToken::Op(op) => op.to_binary(buffer),
            ScriptToken::CheckSigSignature(_) => {
                todo!()
            }
        }
    }

    fn size(&self) -> usize {
        match self {
            ScriptToken::Op(op) => op.size(),
            ScriptToken::CheckSigSignature(_) => 32, // todo: check
        }
    }
}

/// ScriptBuilder can be used to build [Script]s.
pub struct ScriptBuilder {
    /// The tokens.
    ops: Vec<ScriptToken>,
    /// Trailing data to be added at the end of the script. If the script does not end with an
    /// OP_RETURN then it will be inserted during build.
    trailing: Option<Bytes>,
}

impl Default for ScriptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptBuilder {
    /// Create a new ScriptBuilder for constructing a [Script].
    pub fn new() -> ScriptBuilder {
        Self {
            ops: Vec::new(),
            trailing: None,
        }
    }

    /// Build the script.
    pub fn build(&self) -> Result<Script> {
        // initial capacity - 1000 bytes should hold most scripts, unless there's a trailing script
        let cap = match self.trailing.clone() {
            None => 1_000,
            Some(v) => max(1_000, v.len()),
        };
        let mut buffer = Vec::with_capacity(cap);
        let mut last_opreturn = false; // was the last op an OP_RETURN?
        for o in self.ops.iter() {
            o.to_binary(&mut buffer)?;
            last_opreturn = *o == ScriptToken::Op(Operation::OP_RETURN);
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
        self.ops.push(ScriptToken::Op(op));
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
