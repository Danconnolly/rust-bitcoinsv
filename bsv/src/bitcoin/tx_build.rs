use crate::bitcoin::{Tx, TxInput, TxOutput};

/// The TxBuilder builds transactions.
///
/// This is a low-level utility struct and it requires the inputs and outputs to be fully
/// built, including the signatures. What you probably want is a tx_template builder, such as
/// [P2PKHBuilder].
///
/// todo: change this to use traits, declare a TxInputBuilder trait, TxOutputBuilder
pub struct TxBuilder {
    pub version: u32,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub lock_time: u32,
}

impl TxBuilder {
    pub fn new() -> Self {
        Self {
            version: 1,
            inputs: vec![],
            outputs: vec![],
            lock_time: 0,
        }
    }

    pub fn add_input(&mut self, input: &TxInput) -> &mut Self {
        self.inputs.push(input.clone());
        self
    }

    pub fn add_output(&mut self, output: &TxOutput) -> &mut Self {
        self.outputs.push(output.clone());
        self
    }

    pub fn set_lock_time(&mut self, lock_time: u32) -> &mut Self {
        self.lock_time = lock_time;
        self
    }

    pub fn set_version(&mut self, version: u32) -> &mut Self {
        self.version = version;
        self
    }

    pub fn build(&self) -> Tx {
        Tx {
            version: self.version,
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            lock_time: self.lock_time,
        }
    }
}