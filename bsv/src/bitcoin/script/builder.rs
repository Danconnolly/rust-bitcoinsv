use crate::bitcoin::{Encodable, Operation, Outpoint, PrivateKey, Script, TxOutput};
use crate::Error::SigInScript;
use crate::Result;
use bytes::{Buf, BufMut, Bytes};
use std::cmp::max;
use futures::AsyncWriteExt;
use tokio::io::AsyncWriteExt;

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
            ScriptToken::CheckSigSignature(_) => Err(SigInScript),
        }
    }

    fn size(&self) -> usize {
        match self {
            ScriptToken::Op(op) => op.size(),
            ScriptToken::CheckSigSignature(_) => 71,
        }
    }
}

/// ScriptBuilder can be used to build [Script]s.
pub struct ScriptBuilder {
    /// The tokens.
    ops: Vec<ScriptToken>,
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
        }
    }

    /// Add an operation to the script.
    pub fn add(&mut self, op: Operation) -> &mut ScriptBuilder {
        self.ops.push(ScriptToken::Op(op));
        self
    }

    /// Add a signature to the script.
    pub fn add_sig(&mut self, sig: PrivateKey) -> &mut ScriptBuilder {
        self.ops.push(ScriptToken::CheckSigSignature(sig));
        self
    }

    /// Build an output script. An output script cannot contain signatures.
    pub fn build_oscript(&self) -> Result<Script> {
        // initial capacity - 1000 bytes should hold most scripts
        let mut buffer = Vec::with_capacity(1000);
        for o in self.ops.iter() {
            o.to_binary(&mut buffer)?;
        }
        Ok(Script {
            raw: Bytes::from(buffer),
        })
    }

    /// Build an input script.
    ///
    /// We need the tx version and lock time, the outputs, and the outpoints, to generate the pre-image
    /// for signature.
    pub fn build_iscript(
        &self,
        tx_version: u32,
        tx_lock_time: u32,
        outputs: &[TxOutput],
        outpoints: &[Outpoint],
    ) -> Result<Script> {
        // initial capacity - 1000 bytes should hold most scripts
        let mut buffer = Vec::with_capacity(1000);
        for o in self.ops.iter() {
            match o {
                ScriptToken::Op(op) {
                    op.to_binary(&mut buffer)?;
                },
                ScriptToken::CheckSigSignature(sig) => {
                    let pre_image = calc_pre_image(tx_version, tx_lock_time, outputs, outpoints)?;
                    todo!()
                }
            }
        }
        Ok(Script {
            raw: Bytes::from(buffer),
        })
    }
}

/// Calculate the preimage for signature
fn calc_pre_image(tx_version: u32,
                  tx_lock_time: u32,
                  outputs: &[TxOutput],
                  outpoints: &[Outpoint],
) -> Result<Vec<u8>> {
    let mut pre_image = Vec::with_capacity(1000);
    // write version
    let a = pre_image.write_u32_le(tx_version);

    Ok(pre_image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::Operation::OP_PUSH;
    use crate::bitcoin::{ByteSequence, Outpoint, TxHash};
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
            .build_oscript()
            .unwrap();
        assert_eq!(script.raw.len(), 25);
        assert_eq!(
            script.raw,
            Bytes::from(&hex!("76a9146f67988ec4b7bf498c9164d76b52dffdc805ff8c88ac")[..])
        );
    }

    /// Check that building an output script with a signature fails.
    #[test]
    fn check_iscript_fails() {
        // the private key to sign the outpoint being spent
        let (pv_key, _) = PrivateKey::from_wif(&String::from(
            "cTtpACZFWDNTuaEheerpFZyVUBTk7tDFiM4E4xj1joG8sn2Eh8KG",
        ))
        .unwrap();
        let outpoint = Outpoint {
            tx_hash: TxHash::from(
                "85bfc50c0697edeb5f8c3b006fcc889be0ac6ebadcec8097393dd54a92699b60",
            ),
            index: 0,
        };
        let mut script = ScriptBuilder::new();
        script
            .add_sig(pv_key)
            .add(OP_PUSH(ByteSequence::new(Bytes::from(
                "03a2ad2079c0c1bd859e5a4c17e116f1dccfad502dde742802b205868f450f7d93",
            ))));
        let r = script.build_oscript();
        assert!(r.is_err());
    }
}
