// use crate::bitcoin::Operation::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160, OP_PUSH};
// use crate::bitcoin::{
//     Address, ByteSequence, KeyAddressKind, Outpoint, PrivateKey, ScriptBuilder, Tx, TxBuilder,
//     TxInput, TxOutput,
// };
// use bytes::Bytes;
//
// /// Builds a P2PKH transaction.
// ///
// /// A P2PKH transaction spends one or more P2PKH inputs and produces one or more P2PKH outputs.
// pub struct P2PKHBuilder {
//     /// The inputs that are to be spent by the transaction.
//     pub inputs: Vec<P2PKHInput>,
//     /// The outputs that are going to be produced by the transaction.
//     pub outputs: Vec<P2PKHOutput>,
//     /// The locktime for the transaction
//     locktime: u32,
//     /// The kind of blockchain for which this transaction is destined, as detected by addresses.
//     blockchain_kind: Option<KeyAddressKind>,
// }
//
// impl P2PKHBuilder {
//     pub fn new() -> Self {
//         Self {
//             inputs: vec![],
//             outputs: vec![],
//             locktime: 0,
//             blockchain_kind: None,
//         }
//     }
//
//     /// Build the transaction.
//     pub fn build(&self) -> Tx {
//         let mut b = TxBuilder::new();
//         let mut c = b.set_version(1).set_lock_time(self.locktime);
//         for i in &self.inputs {
//             c = c.add_input(&i.build())
//         }
//         for o in &self.outputs {
//             c = c.add_output(&o.build())
//         }
//         c.build()
//     }
//
//     /// Set the locktime for the transaction.
//     pub fn set_locktime(&mut self, locktime: u32) -> &mut Self {
//         self.locktime = locktime;
//         self
//     }
//
//     /// Add a P2PKH input to the transaction.
//     pub fn add_input(&mut self, input: P2PKHInput) -> &mut Self {
//         self.inputs.push(input);
//         self
//     }
//
//     /// Add a P2PKH output to the transaction.
//     pub fn add_output(&mut self, output: P2PKHOutput) -> &mut Self {
//         self.outputs.push(output);
//         self
//     }
//
//     /// During build of the transaction, the type of blockchain can be detected from the addresses used.
//     ///
//     /// This information is used to check that all addresses are for the same kind of blockchain.
//     pub fn get_blockchain_kind(&self) -> Option<KeyAddressKind> {
//         self.blockchain_kind.clone()
//     }
// }
//
// /// A P2PKHInput is an input that spends a P2PKH output.
// pub struct P2PKHInput {
//     /// The outpoint being spent.
//     pub outpoint: Outpoint,
//     /// The private key to unlock the outpoint.
//     pub pv_key: PrivateKey,
// }
//
// impl P2PKHInput {
//     pub fn new(outpoint: Outpoint, pv_key: PrivateKey) -> Self {
//         Self { outpoint, pv_key }
//     }
//
//     pub fn build(&self) -> TxInput {
//         todo!()
//     }
// }
//
// /// A P2PKHOutput is an output that is locked using P2PKH.
// pub struct P2PKHOutput {
//     /// The value of the output.
//     pub value: i64,
//     /// The address to which the value should be sent
//     pub address: Address,
// }
//
// impl P2PKHOutput {
//     pub fn new(value: i64, address: Address) -> Self {
//         Self { value, address }
//     }
//
//     pub fn build(&self) -> TxOutput {
//         let a_hash = ByteSequence::new(Bytes::from(self.address.hash160.hash.to_vec()));
//         let s = ScriptBuilder::new()
//             .add(OP_DUP)
//             .add(OP_HASH160)
//             .add(OP_PUSH(a_hash))
//             .add(OP_EQUALVERIFY)
//             .add(OP_CHECKSIG)
//             .build()
//             .unwrap();
//         TxOutput {
//             value: self.value as u64,
//             script: s,
//         }
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::bitcoin::{AsyncEncodable, TxHash};
//     use hex::ToHex;
//     use std::str::FromStr;
//
//     /// Create an output and check it.
//     #[test]
//     fn test_output_build() {
//         let o = P2PKHOutput::new(
//             90_000_000,
//             Address::from_str("mjPNdfSRh44bxDmB7HkpnBRAF34GJ7wUnc").unwrap(),
//         );
//         let o2 = o.build();
//         let b = o2.to_binary_buf().unwrap();
//         assert_eq!(
//             hex::encode(b),
//             "804a5d05000000001976a9142a717dea82e3040b606daf6afc4f94a54a2b37b788ac"
//         )
//     }
//
//     /// Re-create an existing STN P2PKH transaction.
//     ///
//     /// see notes below labelled "stn tx test" for details of the transaction that was created
//     /// using sv node.
//     #[test]
//     fn stn_tx() {
//         // the hex of the final transaction that should be created
//         let target_hex = "0200000001609b69924ad53d399780ecdcba6eace09b88cc6f003b8c5febed97060cc5bf85000000006b483045022100dbcbac61c96ef2e009a1c851a64c2fed6b3ad148d6637b8a1cc929978bc2cc8d02205c6b52933eb63cabe1c14ea57656e350da5914941a7a11e1d4cb005c15b364c2412103a2ad2079c0c1bd859e5a4c17e116f1dccfad502dde742802b205868f450f7d93feffffff02804a5d05000000001976a9142a717dea82e3040b606daf6afc4f94a54a2b37b788acd8849800000000001976a91406a6c6a3f0663b914ec73d0a4891240726937a6f88ac9ac20000";
//         // the private key to spend the input
//         let input_pvkey = PrivateKey::from_wif(&String::from(
//             "cTtpACZFWDNTuaEheerpFZyVUBTk7tDFiM4E4xj1joG8sn2Eh8KG",
//         ))
//         .unwrap()
//         .0;
//
//         let tx = P2PKHBuilder::new()
//             .set_locktime(49818)
//             .add_input(P2PKHInput::new(
//                 Outpoint {
//                     tx_hash: TxHash::from(
//                         "85bfc50c0697edeb5f8c3b006fcc889be0ac6ebadcec8097393dd54a92699b60",
//                     ),
//                     index: 0,
//                 },
//                 input_pvkey,
//             ))
//             .add_output(P2PKHOutput::new(
//                 90_000_000,
//                 Address::from_str("mjPNdfSRh44bxDmB7HkpnBRAF34GJ7wUnc").unwrap(),
//             ))
//             .add_output(P2PKHOutput::new(
//                 9_995_480,
//                 Address::from_str("mg888zyaVLWEJLUuoAdy3FCV9VvHyzCGEZ").unwrap(),
//             ))
//             .build();
//         // they should be equal
//         assert_eq!(tx.encode_hex::<String>(), target_hex);
//     }
// }
//
// // stn tx test transaction
// // input, address miMK7LsxdSiHYaBQagY4g6E1CshKk8sd6s, value 1, outpoint: 85bfc50c0697edeb5f8c3b006fcc889be0ac6ebadcec8097393dd54a92699b60, index 0
// //   script: "asm": "OP_DUP OP_HASH160 1f1597efb6e913e4b684e5c910818be40f64bd22 OP_EQUALVERIFY OP_CHECKSIG", "hex": "76a9141f1597efb6e913e4b684e5c910818be40f64bd2288ac"
// // send 0.9 to mjPNdfSRh44bxDmB7HkpnBRAF34GJ7wUnc
// //  tx: 386ad6e9ec0e330a6f413e53366b0361b3760555faac471806553595dd3a1976
// //  hex: 0200000001609b69924ad53d399780ecdcba6eace09b88cc6f003b8c5febed97060cc5bf85000000006b483045022100dbcbac61c96ef2e009a1c851a64c2fed6b3ad148d6637b8a1cc929978bc2cc8d02205c6b52933eb63cabe1c14ea57656e350da5914941a7a11e1d4cb005c15b364c2412103a2ad2079c0c1bd859e5a4c17e116f1dccfad502dde742802b205868f450f7d93feffffff02804a5d05000000001976a9142a717dea82e3040b606daf6afc4f94a54a2b37b788acd8849800000000001976a91406a6c6a3f0663b914ec73d0a4891240726937a6f88ac9ac20000
// // bitcoin-cli getrawtransaction 386ad6e9ec0e330a6f413e53366b0361b3760555faac471806553595dd3a1976 1
// // {
// //   "txid": "386ad6e9ec0e330a6f413e53366b0361b3760555faac471806553595dd3a1976",
// //   "hash": "386ad6e9ec0e330a6f413e53366b0361b3760555faac471806553595dd3a1976",
// //   "version": 2,
// //   "size": 226,
// //   "locktime": 49818,
// //   "vin": [
// //     {
// //       "txid": "85bfc50c0697edeb5f8c3b006fcc889be0ac6ebadcec8097393dd54a92699b60",
// //       "vout": 0,
// //       "scriptSig": {
// //         "asm": "3045022100dbcbac61c96ef2e009a1c851a64c2fed6b3ad148d6637b8a1cc929978bc2cc8d02205c6b52933eb63cabe1c14ea57656e350da5914941a7a11e1d4cb005c15b364c2[ALL|FORKID] 03a2ad2079c0c1bd859e5a4c17e116f1dccfad502dde742802b205868f450f7d93",
// //         "hex": "483045022100dbcbac61c96ef2e009a1c851a64c2fed6b3ad148d6637b8a1cc929978bc2cc8d02205c6b52933eb63cabe1c14ea57656e350da5914941a7a11e1d4cb005c15b364c2412103a2ad2079c0c1bd859e5a4c17e116f1dccfad502dde742802b205868f450f7d93"
// //       },
// //       "sequence": 4294967294
// //     }
// //   ],
// //   "vout": [
// //     {
// //       "value": 0.90,
// //       "n": 0,
// //       "scriptPubKey": {
// //         "asm": "OP_DUP OP_HASH160 2a717dea82e3040b606daf6afc4f94a54a2b37b7 OP_EQUALVERIFY OP_CHECKSIG",
// //         "hex": "76a9142a717dea82e3040b606daf6afc4f94a54a2b37b788ac",
// //         "reqSigs": 1,
// //         "type": "pubkeyhash",
// //         "addresses": [
// //           "mjPNdfSRh44bxDmB7HkpnBRAF34GJ7wUnc"
// //         ]
// //       }
// //     },
// //     {
// //       "value": 0.0999548,
// //       "n": 1,
// //       "scriptPubKey": {
// //         "asm": "OP_DUP OP_HASH160 06a6c6a3f0663b914ec73d0a4891240726937a6f OP_EQUALVERIFY OP_CHECKSIG",
// //         "hex": "76a91406a6c6a3f0663b914ec73d0a4891240726937a6f88ac",
// //         "reqSigs": 1,
// //         "type": "pubkeyhash",
// //         "addresses": [
// //           "mg888zyaVLWEJLUuoAdy3FCV9VvHyzCGEZ"
// //         ]
// //       }
// //     }
// //   ],
// //   "hex": "0200000001609b69924ad53d399780ecdcba6eace09b88cc6f003b8c5febed97060cc5bf85000000006b483045022100dbcbac61c96ef2e009a1c851a64c2fed6b3ad148d6637b8a1cc929978bc2cc8d02205c6b52933eb63cabe1c14ea57656e350da5914941a7a11e1d4cb005c15b364c2412103a2ad2079c0c1bd859e5a4c17e116f1dccfad502dde742802b205868f450f7d93feffffff02804a5d05000000001976a9142a717dea82e3040b606daf6afc4f94a54a2b37b788acd8849800000000001976a91406a6c6a3f0663b914ec73d0a4891240726937a6f88ac9ac20000"
// // }
