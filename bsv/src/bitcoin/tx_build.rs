use crate::bitcoin::{Outpoint, ScriptBuilder, Tx, TxInput, TxOutput};
use crate::Result;

/// The TxBuilder builds transactions.
pub struct TxBuilder {
    pub version: u32,
    pub inputs: Vec<TxInputBuilder>,
    pub outputs: Vec<TxOutputBuilder>,
    pub lock_time: u32,
}

impl TxBuilder {
    /// Create a new TxBuilder.
    pub fn new() -> Self {
        Self {
            version: 1,
            inputs: vec![],
            outputs: vec![],
            lock_time: 0,
        }
    }

    /// Add an input.
    ///
    /// Can be used in a chain.
    pub fn add_input(&mut self, input: TxInputBuilder) -> &mut Self {
        self.inputs.push(input);
        self
    }

    /// Add an output.
    ///
    /// Can be used in a chain.
    pub fn add_output(&mut self, output: TxOutputBuilder) -> &mut Self {
        self.outputs.push(output);
        self
    }

    /// Set the lock time for the transaction.
    ///
    /// This is often set to the current block height.
    pub fn set_lock_time(&mut self, lock_time: u32) -> &mut Self {
        self.lock_time = lock_time;
        self
    }

    /// Set the version of the transaction.
    ///
    /// This should never be needed, the default is set correctly.
    pub fn set_version(&mut self, version: u32) -> &mut Self {
        self.version = version;
        self
    }

    /// Build the transaction.
    pub fn build(&self) -> Result<Tx> {
        // build outputs first
        let mut outputs = vec![];
        for o in &self.outputs {
            outputs.push(o.build()?);
        }
        // need the list of outpoints
        let mut outpoints = vec![];
        for i in &self.inputs {
            outpoints.push(i.outpoint.clone());
        }
        let mut inputs = vec![];
        for i in &self.inputs {
            inputs.push(i.build(self.version, self.lock_time, &outputs, &outpoints)?);
        }
        Ok(Tx {
            version: self.version,
            inputs,
            outputs,
            lock_time: self.lock_time,
        })
    }
}

/// The TxInputBuilder builds a single input.
///
/// At the moment we only support SIGHASH_ALL.
pub struct TxInputBuilder {
    pub outpoint: Outpoint,
    pub script: ScriptBuilder,
    pub sequence: u32,
}

impl TxInputBuilder {
    /// Create a new input builder.
    pub fn new(outpoint: Outpoint, script: ScriptBuilder) -> Self {
        TxInputBuilder {
            outpoint,
            script,
            sequence: 0xffffffff,
        }
    }

    /// Set the sequence value. This will rarely be needed.
    pub fn set_sequence(&mut self, sequence: u32) -> &mut Self {
        self.sequence = sequence;
        self
    }

    /// Build the input. The input can only be built when all the parameters of the transaction are
    /// defined.
    ///
    /// To build an input we need the transaction parameters (version & locktime),
    /// all of the encoded outputs, and the outpoints of the inputs.
    pub fn build(
        &self,
        tx_version: u32,
        tx_lock_time: u32,
        outputs: &[TxOutput],
        outpoints: &[Outpoint],
    ) -> Result<TxInput> {
        Ok(TxInput {
            outpoint: self.outpoint.clone(),
            script: self
                .script
                .build_iscript(tx_version, tx_lock_time, outputs, outpoints)?,
            sequence: self.sequence,
        })
    }
}

/// The TxOutputBuilder builds a single output.
pub struct TxOutputBuilder {
    pub value: i64,
    pub script: ScriptBuilder,
}

impl TxOutputBuilder {
    pub fn new(value: i64, script: ScriptBuilder) -> Self {
        TxOutputBuilder { value, script }
    }

    /// Build the output.
    ///
    /// Outputs dont contain signatures, so these are simpler.
    pub fn build(&self) -> Result<TxOutput> {
        Ok(TxOutput {
            value: self.value as u64,
            script: self.script.build_oscript()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::Operation::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160, OP_PUSH};
    use crate::bitcoin::{
        Address, AsyncEncodable, ByteSequence, PrivateKey, ScriptBuilder, TxHash,
    };
    use bytes::Bytes;
    use std::str::FromStr;

    /// Manually create a P2PKH input and check we have as much as possible.
    ///
    /// The parameters for this test were imported from STN transaction 386ad6e9ec0e330a6f413e53366b0361b3760555faac471806553595dd3a1976.
    #[test]
    fn test_input_build() {
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
        // let mut ib = TxInputBuilder::new(outpoint, script); todo
        // let i = ib.build().unwrap();
        // let b = i.to_binary_buf().unwrap();
        // // todo: hex string below is script only
        // assert_eq!(hex::encode(b), "483045022100dbcbac61c96ef2e009a1c851a64c2fed6b3ad148d6637b8a1cc929978bc2cc8d02205c6b52933eb63cabe1c14ea57656e350da5914941a7a11e1d4cb005c15b364c2412103a2ad2079c0c1bd859e5a4c17e116f1dccfad502dde742802b205868f450f7d93");
    }

    /// Manually create a P2PKH output and check it.
    ///
    /// The parameters for this test were imported from STN transaction 386ad6e9ec0e330a6f413e53366b0361b3760555faac471806553595dd3a1976.
    #[test]
    fn test_output_build() {
        let a = Address::from_str("mjPNdfSRh44bxDmB7HkpnBRAF34GJ7wUnc").unwrap();
        let a_hash = ByteSequence::new(Bytes::from(a.hash160.hash.to_vec()));
        let mut b = ScriptBuilder::new();
        b.add(OP_DUP)
            .add(OP_HASH160)
            .add(OP_PUSH(a_hash))
            .add(OP_EQUALVERIFY)
            .add(OP_CHECKSIG);
        let o = TxOutputBuilder::new(90_000_000, b);
        let o2 = o.build().unwrap();
        let b = o2.to_binary_buf().unwrap();
        assert_eq!(
            hex::encode(b),
            "804a5d05000000001976a9142a717dea82e3040b606daf6afc4f94a54a2b37b788ac"
        )
    }
}
