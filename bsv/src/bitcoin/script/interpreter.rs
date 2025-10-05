use crate::bitcoin::script::{verify_signature, Operation, Script};
use crate::bitcoin::{Hash, Tx};
use crate::{Error, Result};
use bytes::Bytes;
use ripemd::{Digest as RipemdDigest, Ripemd160};
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::{Digest as Sha2Digest, Sha256};
use std::collections::VecDeque;

/// Maximum script size in bytes
const MAX_SCRIPT_SIZE: usize = 10_000;

/// Maximum number of operations in a script
const MAX_SCRIPT_OPS: usize = 201;

/// Maximum stack size
const MAX_STACK_SIZE: usize = 1000;

/// Maximum number size in bytes for script numbers
const MAX_NUM_SIZE: usize = 4;

/// Transaction context for script evaluation
pub struct TransactionContext<'a> {
    pub tx: &'a Tx,
    pub input_index: usize,
    pub subscript: &'a [u8],
}

/// Script interpreter for evaluating Bitcoin scripts
pub struct ScriptInterpreter {
    pub(crate) main_stack: VecDeque<Bytes>,
    pub(crate) alt_stack: VecDeque<Bytes>,
    pub(crate) if_stack: Vec<bool>,
    pub(crate) op_count: usize,
}

impl Default for ScriptInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptInterpreter {
    pub fn new() -> Self {
        Self {
            main_stack: VecDeque::new(),
            alt_stack: VecDeque::new(),
            if_stack: Vec::new(),
            op_count: 0,
        }
    }

    /// Evaluate a script without transaction context
    pub fn eval_script(&mut self, script: &Script) -> Result<bool> {
        self.eval_script_with_context(script, None)
    }

    /// Evaluate a script with optional transaction context
    pub fn eval_script_with_context(
        &mut self,
        script: &Script,
        context: Option<&TransactionContext>,
    ) -> Result<bool> {
        if script.len() > MAX_SCRIPT_SIZE {
            return Err(Error::ScriptTooLarge);
        }

        let ops = script.operations()?;

        for op in ops {
            if self.op_count > MAX_SCRIPT_OPS {
                return Err(Error::ScriptTooManyOps);
            }

            // Handle flow control operations regardless of execution state
            match op {
                Operation::OP_IF => {
                    let condition = if self.is_branch_executing() {
                        if self.main_stack.is_empty() {
                            false
                        } else {
                            let top = self.pop_bytes()?;
                            bytes_to_bool(&top)
                        }
                    } else {
                        false
                    };
                    self.if_stack.push(condition);
                    self.op_count += 1;
                    continue;
                }
                Operation::OP_NOTIF => {
                    let condition = if self.is_branch_executing() {
                        if self.main_stack.is_empty() {
                            true
                        } else {
                            let top = self.pop_bytes()?;
                            !bytes_to_bool(&top)
                        }
                    } else {
                        false
                    };
                    self.if_stack.push(condition);
                    self.op_count += 1;
                    continue;
                }
                Operation::OP_ELSE => {
                    if self.if_stack.is_empty() {
                        return Err(Error::ScriptUnbalancedConditional);
                    }
                    let last = self.if_stack.len() - 1;
                    self.if_stack[last] = !self.if_stack[last];
                    self.op_count += 1;
                    continue;
                }
                Operation::OP_ENDIF => {
                    if self.if_stack.is_empty() {
                        return Err(Error::ScriptUnbalancedConditional);
                    }
                    self.if_stack.pop();
                    self.op_count += 1;
                    continue;
                }
                _ => {}
            }

            // Skip non-flow-control operations in unexecuted branches
            if !self.is_branch_executing() {
                continue;
            }

            self.execute_op(&op, context)?;
            self.op_count += 1;

            if self.main_stack.len() + self.alt_stack.len() > MAX_STACK_SIZE {
                return Err(Error::ScriptStackOverflow);
            }
        }

        if !self.if_stack.is_empty() {
            return Err(Error::ScriptUnbalancedConditional);
        }

        Ok(!self.main_stack.is_empty() && self.stack_top_true())
    }

    /// Execute a single operation
    fn execute_op(&mut self, op: &Operation, context: Option<&TransactionContext>) -> Result<()> {
        use Operation::*;

        match op {
            // Push value operations
            OP_0 | OP_FALSE => self.push_bytes(Bytes::from_static(&[0])),
            OP_1NEGATE => self.push_bytes(Bytes::from_static(&[0x81])),
            OP_1 | OP_TRUE => self.push_bytes(Bytes::from_static(&[1])),
            OP_2 => self.push_bytes(Bytes::from_static(&[2])),
            OP_3 => self.push_bytes(Bytes::from_static(&[3])),
            OP_4 => self.push_bytes(Bytes::from_static(&[4])),
            OP_5 => self.push_bytes(Bytes::from_static(&[5])),
            OP_6 => self.push_bytes(Bytes::from_static(&[6])),
            OP_7 => self.push_bytes(Bytes::from_static(&[7])),
            OP_8 => self.push_bytes(Bytes::from_static(&[8])),
            OP_9 => self.push_bytes(Bytes::from_static(&[9])),
            OP_10 => self.push_bytes(Bytes::from_static(&[10])),
            OP_11 => self.push_bytes(Bytes::from_static(&[11])),
            OP_12 => self.push_bytes(Bytes::from_static(&[12])),
            OP_13 => self.push_bytes(Bytes::from_static(&[13])),
            OP_14 => self.push_bytes(Bytes::from_static(&[14])),
            OP_15 => self.push_bytes(Bytes::from_static(&[15])),
            OP_16 => self.push_bytes(Bytes::from_static(&[16])),

            OP_PUSH(data) | OP_PUSHDATA1(data) | OP_PUSHDATA2(data) | OP_PUSHDATA4(data) => {
                self.push_bytes(data.get_bytes())
            }

            // Flow control
            OP_NOP => {}
            OP_VERIFY => {
                if self.main_stack.is_empty() || !self.stack_top_true() {
                    return Err(Error::ScriptVerifyFailed);
                }
                self.pop_bytes()?;
            }
            OP_RETURN => return Err(Error::ScriptOpReturn),

            // Stack operations
            OP_TOALTSTACK => {
                let item = self.pop_bytes()?;
                self.alt_stack.push_back(item);
            }
            OP_FROMALTSTACK => {
                if self.alt_stack.is_empty() {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let item = self.alt_stack.pop_back().unwrap();
                self.push_bytes(item);
            }
            OP_IFDUP => {
                if !self.main_stack.is_empty() && bytes_to_bool(self.main_stack.back().unwrap()) {
                    let top = self.main_stack.back().unwrap().clone();
                    self.push_bytes(top);
                }
            }
            OP_DEPTH => {
                let depth = self.main_stack.len();
                self.push_int(depth as i64)?;
            }
            OP_DROP => {
                self.pop_bytes()?;
            }
            OP_DUP => {
                if self.main_stack.is_empty() {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let top = self.main_stack.back().unwrap().clone();
                self.push_bytes(top);
            }
            OP_NIP => {
                self.remove_at(1)?;
            }
            OP_OVER => {
                if self.main_stack.len() < 2 {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let item = self.main_stack[self.main_stack.len() - 2].clone();
                self.push_bytes(item);
            }
            OP_PICK => {
                let n = self.pop_int()? as usize;
                if n >= self.main_stack.len() {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let item = self.main_stack[self.main_stack.len() - n - 1].clone();
                self.push_bytes(item);
            }
            OP_ROLL => {
                let n = self.pop_int()? as usize;
                if n >= self.main_stack.len() {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let idx = self.main_stack.len() - n - 1;
                let item = self.main_stack.remove(idx).unwrap();
                self.push_bytes(item);
            }
            OP_ROT => {
                if self.main_stack.len() < 3 {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let len = self.main_stack.len();
                self.main_stack.swap(len - 3, len - 2);
                self.main_stack.swap(len - 2, len - 1);
            }
            OP_SWAP => {
                if self.main_stack.len() < 2 {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let len = self.main_stack.len();
                self.main_stack.swap(len - 2, len - 1);
            }
            OP_TUCK => {
                if self.main_stack.len() < 2 {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let top = self.main_stack.back().unwrap().clone();
                self.main_stack.insert(self.main_stack.len() - 2, top);
            }
            OP_2DROP => {
                self.pop_bytes()?;
                self.pop_bytes()?;
            }
            OP_2DUP => {
                if self.main_stack.len() < 2 {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let a = self.main_stack[self.main_stack.len() - 2].clone();
                let b = self.main_stack[self.main_stack.len() - 1].clone();
                self.push_bytes(a);
                self.push_bytes(b);
            }
            OP_3DUP => {
                if self.main_stack.len() < 3 {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let a = self.main_stack[self.main_stack.len() - 3].clone();
                let b = self.main_stack[self.main_stack.len() - 2].clone();
                let c = self.main_stack[self.main_stack.len() - 1].clone();
                self.push_bytes(a);
                self.push_bytes(b);
                self.push_bytes(c);
            }
            OP_2OVER => {
                if self.main_stack.len() < 4 {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let a = self.main_stack[self.main_stack.len() - 4].clone();
                let b = self.main_stack[self.main_stack.len() - 3].clone();
                self.push_bytes(a);
                self.push_bytes(b);
            }
            OP_2ROT => {
                if self.main_stack.len() < 6 {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let len = self.main_stack.len();
                let a = self.main_stack.remove(len - 6).unwrap();
                let b = self.main_stack.remove(len - 6).unwrap();
                self.push_bytes(a);
                self.push_bytes(b);
            }
            OP_2SWAP => {
                if self.main_stack.len() < 4 {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let len = self.main_stack.len();
                self.main_stack.swap(len - 4, len - 2);
                self.main_stack.swap(len - 3, len - 1);
            }

            // Splice operations
            OP_SIZE => {
                if self.main_stack.is_empty() {
                    return Err(Error::ScriptInvalidStackOperation);
                }
                let size = self.main_stack.back().unwrap().len();
                self.push_int(size as i64)?;
            }

            // Bitwise logic
            OP_EQUAL => {
                let b = self.pop_bytes()?;
                let a = self.pop_bytes()?;
                self.push_bool(a == b);
            }
            OP_EQUALVERIFY => {
                let b = self.pop_bytes()?;
                let a = self.pop_bytes()?;
                if a != b {
                    return Err(Error::ScriptVerifyFailed);
                }
            }

            // Arithmetic
            OP_1ADD => {
                let n = self.pop_int()?;
                self.push_int(n + 1)?;
            }
            OP_1SUB => {
                let n = self.pop_int()?;
                self.push_int(n - 1)?;
            }
            OP_NEGATE => {
                let n = self.pop_int()?;
                self.push_int(-n)?;
            }
            OP_ABS => {
                let n = self.pop_int()?;
                self.push_int(n.abs())?;
            }
            OP_NOT => {
                let n = self.pop_int()?;
                self.push_bool(n == 0);
            }
            OP_0NOTEQUAL => {
                let n = self.pop_int()?;
                self.push_bool(n != 0);
            }
            OP_ADD => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_int(a + b)?;
            }
            OP_SUB => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_int(a - b)?;
            }
            OP_BOOLAND => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_bool(a != 0 && b != 0);
            }
            OP_BOOLOR => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_bool(a != 0 || b != 0);
            }
            OP_NUMEQUAL => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_bool(a == b);
            }
            OP_NUMEQUALVERIFY => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                if a != b {
                    return Err(Error::ScriptVerifyFailed);
                }
            }
            OP_NUMNOTEQUAL => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_bool(a != b);
            }
            OP_LESSTHAN => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_bool(a < b);
            }
            OP_GREATERTHAN => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_bool(a > b);
            }
            OP_LESSTHANOREQUAL => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_bool(a <= b);
            }
            OP_GREATERTHANOREQUAL => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_bool(a >= b);
            }
            OP_MIN => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_int(a.min(b))?;
            }
            OP_MAX => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push_int(a.max(b))?;
            }
            OP_WITHIN => {
                let max = self.pop_int()?;
                let min = self.pop_int()?;
                let x = self.pop_int()?;
                self.push_bool(x >= min && x < max);
            }

            // Crypto operations
            OP_RIPEMD160 => {
                let data = self.pop_bytes()?;
                let mut hasher = Ripemd160::new();
                hasher.update(&data);
                let result = hasher.finalize();
                self.push_bytes(Bytes::copy_from_slice(&result));
            }
            OP_SHA1 => {
                let data = self.pop_bytes()?;
                let mut hasher = Sha1::new();
                Sha1Digest::update(&mut hasher, &data);
                let result = hasher.finalize();
                self.push_bytes(Bytes::copy_from_slice(&result));
            }
            OP_SHA256 => {
                let data = self.pop_bytes()?;
                let mut hasher = Sha256::new();
                Sha2Digest::update(&mut hasher, &data);
                let result = hasher.finalize();
                self.push_bytes(Bytes::copy_from_slice(&result));
            }
            OP_HASH160 => {
                let data = self.pop_bytes()?;
                let mut sha_hasher = Sha256::new();
                Sha2Digest::update(&mut sha_hasher, &data);
                let sha_result = sha_hasher.finalize();

                let mut ripemd_hasher = Ripemd160::new();
                RipemdDigest::update(&mut ripemd_hasher, sha_result);
                let result = ripemd_hasher.finalize();
                self.push_bytes(Bytes::copy_from_slice(&result));
            }
            OP_HASH256 => {
                let data = self.pop_bytes()?;
                let hash = Hash::sha256d(&data);
                self.push_bytes(hash.encode_bytes());
            }

            // Disabled/unimplemented operations
            OP_CAT | OP_SPLIT | OP_AND | OP_OR | OP_XOR | OP_INVERT | OP_2MUL | OP_2DIV
            | OP_MUL | OP_DIV | OP_MOD | OP_LSHIFT | OP_RSHIFT => {
                return Err(Error::ScriptDisabledOpcode);
            }

            // Signature operations
            OP_CHECKSIG => {
                if context.is_none() {
                    return Err(Error::ScriptRequiresContext);
                }
                let ctx = context.unwrap();

                // Pop public key and signature
                let pubkey = self.pop_bytes()?;
                let sig = self.pop_bytes()?;

                // Verify signature
                let valid = if sig.is_empty() {
                    false
                } else {
                    verify_signature(&sig, &pubkey, ctx.tx, ctx.input_index, ctx.subscript)?
                };

                self.push_bool(valid);
            }
            OP_CHECKSIGVERIFY => {
                if context.is_none() {
                    return Err(Error::ScriptRequiresContext);
                }
                let ctx = context.unwrap();

                // Pop public key and signature
                let pubkey = self.pop_bytes()?;
                let sig = self.pop_bytes()?;

                // Verify signature
                let valid = if sig.is_empty() {
                    false
                } else {
                    verify_signature(&sig, &pubkey, ctx.tx, ctx.input_index, ctx.subscript)?
                };

                if !valid {
                    return Err(Error::ScriptVerifyFailed);
                }
            }

            // Other operations that need transaction context (not implemented)
            OP_CHECKMULTISIG | OP_CHECKMULTISIGVERIFY | OP_CODESEPARATOR => {
                return Err(Error::ScriptRequiresContext);
            }

            // Flow control operations (should be handled in eval_script main loop)
            OP_IF | OP_NOTIF | OP_ELSE | OP_ENDIF => {
                return Err(Error::Internal(
                    "Flow control operations should be handled in eval_script".to_string(),
                ));
            }

            // Reserved operations
            OP_RESERVED | OP_VER | OP_VERIF | OP_VERNOTIF => {
                return Err(Error::ScriptReservedOpcode);
            }

            // Unimplemented operations
            OP_NUM2BIN | OP_BIN2NUM | OP_UPNOP => {
                return Err(Error::ScriptUnimplementedOpcode);
            }
        }

        Ok(())
    }

    /// Check if current branch is executing
    fn is_branch_executing(&self) -> bool {
        self.if_stack.iter().all(|&b| b)
    }

    /// Check if stack top is true
    pub(crate) fn stack_top_true(&self) -> bool {
        if let Some(top) = self.main_stack.back() {
            bytes_to_bool(top)
        } else {
            false
        }
    }

    /// Push bytes onto the main stack
    pub(crate) fn push_bytes(&mut self, data: Bytes) {
        self.main_stack.push_back(data);
    }

    /// Pop bytes from the main stack
    pub(crate) fn pop_bytes(&mut self) -> Result<Bytes> {
        self.main_stack
            .pop_back()
            .ok_or(Error::ScriptInvalidStackOperation)
    }

    /// Push a boolean value
    pub(crate) fn push_bool(&mut self, val: bool) {
        self.push_bytes(if val {
            Bytes::from_static(&[1])
        } else {
            Bytes::from_static(&[])
        });
    }

    /// Push an integer
    fn push_int(&mut self, val: i64) -> Result<()> {
        self.push_bytes(int_to_bytes(val)?);
        Ok(())
    }

    /// Pop an integer
    fn pop_int(&mut self) -> Result<i64> {
        let bytes = self.pop_bytes()?;
        bytes_to_int(&bytes)
    }

    /// Remove item at index from the end
    fn remove_at(&mut self, idx: usize) -> Result<()> {
        if idx >= self.main_stack.len() {
            return Err(Error::ScriptInvalidStackOperation);
        }
        let pos = self.main_stack.len() - idx - 1;
        self.main_stack.remove(pos);
        Ok(())
    }

    /// Clear the interpreter state
    pub fn clear(&mut self) {
        self.main_stack.clear();
        self.alt_stack.clear();
        self.if_stack.clear();
        self.op_count = 0;
    }
}

/// Convert bytes to boolean (empty or all zeros is false)
fn bytes_to_bool(bytes: &[u8]) -> bool {
    for (i, &byte) in bytes.iter().enumerate() {
        if byte != 0 {
            // Negative zero is still false
            if i == bytes.len() - 1 && byte == 0x80 {
                return false;
            }
            return true;
        }
    }
    false
}

/// Convert bytes to integer
fn bytes_to_int(bytes: &[u8]) -> Result<i64> {
    if bytes.is_empty() {
        return Ok(0);
    }

    if bytes.len() > MAX_NUM_SIZE {
        return Err(Error::ScriptNumberTooLarge);
    }

    let mut result = 0i64;
    for (i, &byte) in bytes.iter().enumerate() {
        result |= (byte as i64) << (8 * i);
    }

    // Handle sign bit
    if bytes[bytes.len() - 1] & 0x80 != 0 {
        result &= !(0x80_i64 << (8 * (bytes.len() - 1)));
        result = -result;
    }

    Ok(result)
}

/// Convert integer to bytes (minimal encoding)
fn int_to_bytes(val: i64) -> Result<Bytes> {
    if val == 0 {
        return Ok(Bytes::new());
    }

    let mut bytes = Vec::new();
    let negative = val < 0;
    let mut abs_val = val.unsigned_abs();

    while abs_val > 0 {
        bytes.push((abs_val & 0xff) as u8);
        abs_val >>= 8;
    }

    // Add sign bit if necessary
    if bytes[bytes.len() - 1] & 0x80 != 0 {
        bytes.push(if negative { 0x80 } else { 0 });
    } else if negative {
        let last = bytes.len() - 1;
        bytes[last] |= 0x80;
    }

    Ok(Bytes::from(bytes))
}

/// Verify a transaction input against its referenced output script
pub fn verify_script(
    script_sig: &Script,
    script_pubkey: &Script,
    tx: &Tx,
    input_index: usize,
) -> Result<bool> {
    let mut interpreter = ScriptInterpreter::new();

    // Execute the signature script (no context needed for sig script)
    if !interpreter.eval_script(script_sig)? {
        return Ok(false);
    }

    // Copy the stack for P2SH evaluation
    let _stack_copy = interpreter.main_stack.clone();

    // Create context for pubkey script evaluation
    let context = TransactionContext {
        tx,
        input_index,
        subscript: &script_pubkey.raw,
    };

    // Execute the pubkey script with context
    let result = interpreter.eval_script_with_context(script_pubkey, Some(&context))?;

    // TODO: Handle P2SH evaluation if applicable

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::script::builder::ScriptBuilder;

    #[test]
    fn test_push_operations() {
        let mut interpreter = ScriptInterpreter::new();

        // Test pushing numbers
        interpreter.execute_op(&Operation::OP_1, None).unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), 1);

        interpreter.execute_op(&Operation::OP_16, None).unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), 16);

        interpreter
            .execute_op(&Operation::OP_1NEGATE, None)
            .unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), -1);
    }

    #[test]
    fn test_stack_operations() {
        let mut interpreter = ScriptInterpreter::new();

        // DUP
        interpreter.push_int(5).unwrap();
        interpreter.execute_op(&Operation::OP_DUP, None).unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), 5);
        assert_eq!(interpreter.pop_int().unwrap(), 5);

        // SWAP
        interpreter.push_int(1).unwrap();
        interpreter.push_int(2).unwrap();
        interpreter.execute_op(&Operation::OP_SWAP, None).unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), 1);
        assert_eq!(interpreter.pop_int().unwrap(), 2);

        // DROP
        interpreter.push_int(42).unwrap();
        interpreter.execute_op(&Operation::OP_DROP, None).unwrap();
        assert!(interpreter.main_stack.is_empty());
    }

    #[test]
    fn test_arithmetic_operations() {
        let mut interpreter = ScriptInterpreter::new();

        // ADD
        interpreter.push_int(3).unwrap();
        interpreter.push_int(5).unwrap();
        interpreter.execute_op(&Operation::OP_ADD, None).unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), 8);

        // SUB
        interpreter.push_int(10).unwrap();
        interpreter.push_int(3).unwrap();
        interpreter.execute_op(&Operation::OP_SUB, None).unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), 7);

        // 1ADD
        interpreter.push_int(5).unwrap();
        interpreter.execute_op(&Operation::OP_1ADD, None).unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), 6);

        // NEGATE
        interpreter.push_int(5).unwrap();
        interpreter.execute_op(&Operation::OP_NEGATE, None).unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), -5);

        // ABS
        interpreter.push_int(-5).unwrap();
        interpreter.execute_op(&Operation::OP_ABS, None).unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), 5);
    }

    #[test]
    fn test_comparison_operations() {
        let mut interpreter = ScriptInterpreter::new();

        // EQUAL
        interpreter.push_int(5).unwrap();
        interpreter.push_int(5).unwrap();
        interpreter
            .execute_op(&Operation::OP_NUMEQUAL, None)
            .unwrap();
        assert!(interpreter.pop_int().unwrap() == 1);

        // LESSTHAN
        interpreter.push_int(3).unwrap();
        interpreter.push_int(5).unwrap();
        interpreter
            .execute_op(&Operation::OP_LESSTHAN, None)
            .unwrap();
        assert!(interpreter.pop_int().unwrap() == 1);

        // GREATERTHAN
        interpreter.push_int(5).unwrap();
        interpreter.push_int(3).unwrap();
        interpreter
            .execute_op(&Operation::OP_GREATERTHAN, None)
            .unwrap();
        assert!(interpreter.pop_int().unwrap() == 1);
    }

    #[test]
    fn test_hash_operations() {
        let mut interpreter = ScriptInterpreter::new();

        // SHA256
        interpreter.push_bytes(Bytes::from("hello"));
        interpreter.execute_op(&Operation::OP_SHA256, None).unwrap();
        let hash = interpreter.pop_bytes().unwrap();
        assert_eq!(hash.len(), 32);

        // HASH160
        interpreter.push_bytes(Bytes::from("hello"));
        interpreter
            .execute_op(&Operation::OP_HASH160, None)
            .unwrap();
        let hash = interpreter.pop_bytes().unwrap();
        assert_eq!(hash.len(), 20);

        // HASH256 (double SHA256)
        interpreter.push_bytes(Bytes::from("hello"));
        interpreter
            .execute_op(&Operation::OP_HASH256, None)
            .unwrap();
        let hash = interpreter.pop_bytes().unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_flow_control() {
        // IF-ELSE-ENDIF with true condition
        let script = ScriptBuilder::new()
            .add(Operation::OP_1)
            .add(Operation::OP_IF)
            .add(Operation::OP_10)
            .add(Operation::OP_ELSE)
            .add(Operation::OP_15) // This should not execute
            .add(Operation::OP_ENDIF)
            .build()
            .unwrap();
        let mut interpreter = ScriptInterpreter::new();
        assert!(interpreter.eval_script(&script).unwrap());
        assert_eq!(interpreter.pop_int().unwrap(), 10);

        // IF-ELSE-ENDIF with false condition
        let script = ScriptBuilder::new()
            .add(Operation::OP_0)
            .add(Operation::OP_IF)
            .add(Operation::OP_10) // This should not execute
            .add(Operation::OP_ELSE)
            .add(Operation::OP_15)
            .add(Operation::OP_ENDIF)
            .build()
            .unwrap();
        let mut interpreter = ScriptInterpreter::new();
        assert!(interpreter.eval_script(&script).unwrap());
        assert_eq!(interpreter.pop_int().unwrap(), 15);

        // Nested IF
        let script = ScriptBuilder::new()
            .add(Operation::OP_1)
            .add(Operation::OP_IF)
            .add(Operation::OP_1)
            .add(Operation::OP_IF)
            .add(Operation::OP_7)
            .add(Operation::OP_ENDIF)
            .add(Operation::OP_ENDIF)
            .build()
            .unwrap();
        let mut interpreter = ScriptInterpreter::new();
        assert!(interpreter.eval_script(&script).unwrap());
        assert_eq!(interpreter.pop_int().unwrap(), 7);
    }

    #[test]
    fn test_verify_operations() {
        let mut interpreter = ScriptInterpreter::new();

        // VERIFY with true value
        interpreter.push_int(1).unwrap();
        assert!(interpreter.execute_op(&Operation::OP_VERIFY, None).is_ok());

        // VERIFY with false value
        interpreter.push_int(0).unwrap();
        assert!(interpreter.execute_op(&Operation::OP_VERIFY, None).is_err());

        // EQUALVERIFY with equal values
        interpreter.clear();
        interpreter.push_int(5).unwrap();
        interpreter.push_int(5).unwrap();
        assert!(interpreter
            .execute_op(&Operation::OP_EQUALVERIFY, None)
            .is_ok());

        // EQUALVERIFY with unequal values
        interpreter.push_int(5).unwrap();
        interpreter.push_int(6).unwrap();
        assert!(interpreter
            .execute_op(&Operation::OP_EQUALVERIFY, None)
            .is_err());
    }

    #[test]
    fn test_alt_stack() {
        let mut interpreter = ScriptInterpreter::new();

        // Move to alt stack and back
        interpreter.push_int(42).unwrap();
        interpreter
            .execute_op(&Operation::OP_TOALTSTACK, None)
            .unwrap();
        assert!(interpreter.main_stack.is_empty());

        interpreter
            .execute_op(&Operation::OP_FROMALTSTACK, None)
            .unwrap();
        assert_eq!(interpreter.pop_int().unwrap(), 42);
    }

    #[test]
    fn test_script_evaluation() {
        // Test simple true script
        let script = ScriptBuilder::new().add(Operation::OP_1).build().unwrap();
        let mut interpreter = ScriptInterpreter::new();
        assert!(interpreter.eval_script(&script).unwrap());

        // Test simple false script
        let script = ScriptBuilder::new().add(Operation::OP_0).build().unwrap();
        let mut interpreter = ScriptInterpreter::new();
        assert!(!interpreter.eval_script(&script).unwrap());

        // Test arithmetic evaluation
        let script = ScriptBuilder::new()
            .add(Operation::OP_2)
            .add(Operation::OP_3)
            .add(Operation::OP_ADD)
            .add(Operation::OP_5)
            .add(Operation::OP_EQUAL)
            .build()
            .unwrap();
        let mut interpreter = ScriptInterpreter::new();
        assert!(interpreter.eval_script(&script).unwrap());
    }

    #[test]
    fn test_bytes_conversion() {
        // Test integer to bytes and back
        assert_eq!(bytes_to_int(&int_to_bytes(0).unwrap()).unwrap(), 0);
        assert_eq!(bytes_to_int(&int_to_bytes(1).unwrap()).unwrap(), 1);
        assert_eq!(bytes_to_int(&int_to_bytes(-1).unwrap()).unwrap(), -1);
        assert_eq!(bytes_to_int(&int_to_bytes(127).unwrap()).unwrap(), 127);
        assert_eq!(bytes_to_int(&int_to_bytes(-127).unwrap()).unwrap(), -127);
        assert_eq!(bytes_to_int(&int_to_bytes(128).unwrap()).unwrap(), 128);
        assert_eq!(bytes_to_int(&int_to_bytes(-128).unwrap()).unwrap(), -128);
        assert_eq!(bytes_to_int(&int_to_bytes(32767).unwrap()).unwrap(), 32767);
        assert_eq!(
            bytes_to_int(&int_to_bytes(-32767).unwrap()).unwrap(),
            -32767
        );
    }

    #[test]
    fn test_boolean_conversion() {
        assert!(!bytes_to_bool(&[]));
        assert!(!bytes_to_bool(&[0]));
        assert!(!bytes_to_bool(&[0, 0]));
        assert!(!bytes_to_bool(&[0x80])); // negative zero
        assert!(bytes_to_bool(&[1]));
        assert!(bytes_to_bool(&[0x81])); // negative one
        assert!(bytes_to_bool(&[0, 1]));
    }
}
