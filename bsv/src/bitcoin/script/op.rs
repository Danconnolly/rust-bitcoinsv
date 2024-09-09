use bytes::{Buf, BufMut, Bytes};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use crate::{BsvError, BsvResult};
use crate::bitcoin::encoding::Encodable;

// todo:
// Pushes 0 onto the stack
// OP_FALSE= 0;
// Pushes 1 onto the stack
// pub const OP_TRUE= 81;


/// An Operation is an opcode plus relevant data.
///
/// todo: add Copy trait
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]      // we want to keep the Bitcoin standard naming convention
#[repr(u8)]
pub enum Operation {
    /// Pushes 0 onto the stack.
    OP_0,
    /// Pushes data onto the stack where the data must be 1-75 bytes long.
    OP_PUSH(Bytes),
    /// The next byte sets the number of bytes to push onto the stack
    OP_PUSHDATA1(Bytes),
    /// The next two bytes sets the number of bytes to push onto the stack
    OP_PUSHDATA2(Bytes),
    /// The next four bytes sets the number of bytes to push onto the stack
    OP_PUSHDATA4(Bytes),
    /// Pushes -1 onto the stack
    OP_1NEGATE = 79,
    /// Pushes 1 onto the stack
    OP_1 = 81,
    /// Pushes 2 onto the stack
    OP_2 = 82,
    /// Pushes 3 onto the stack
    OP_3 = 83,
    /// Pushes 4 onto the stack
    OP_4 = 84,
    /// Pushes 5 onto the stack
    OP_5 = 85,
    /// Pushes 6 onto the stack
    OP_6 = 86,
    /// Pushes 7 onto the stack
    OP_7 = 87,
    /// Pushes 8 onto the stack
    OP_8 = 88,
    /// Pushes 9 onto the stack
    OP_9 = 89,
    /// Pushes 10 onto the stack
    OP_10 = 90,
    /// Pushes 11 onto the stack
    OP_11 = 91,
    /// Pushes 12 onto the stack
    OP_12 = 92,
    /// Pushes 13 onto the stack
    OP_13 = 93,
    /// Pushes 14 onto the stack
    OP_14 = 94,
    /// Pushes 15 onto the stack
    OP_15 = 95,
    /// Pushes 16 onto the stack
    OP_16 = 96,

    // --------------------------------------------------------------------------------------------
    // Flow Control
    // --------------------------------------------------------------------------------------------
    
    /// Does nothing
    OP_NOP = 97,
    /// If the top stack is true, statements are executed. Top stack value is removed.
    OP_IF = 99,
    /// If the top stack is false, statements are executed. Top stack value is removed.
    OP_NOTIF = 100,
    /// If the preceding OP_IF or OP_NOTIF statemetns were not executed, then statements are executed.
    OP_ELSE = 103,
    /// Ends an if-else block
    OP_ENDIF = 104,
    /// Marks a statement as invalid if the top stack value is false. Top stack value is removed.
    OP_VERIFY = 105,
    /// Marks a statements as invalid
    OP_RETURN = 106,
    
    // --------------------------------------------------------------------------------------------
    // Stack
    // --------------------------------------------------------------------------------------------
    
    /// Moves the top item on the main stack to the alt stack
    OP_TOALTSTACK = 107,
    /// Moves the top item on the alt stack to the main stack
    OP_FROMALTSTACK = 108,
    /// Duplicates the top stack value if it is not zero
    OP_IFDUP = 115,
    /// Puts the number of stack items onto the stack
    OP_DEPTH = 116,
    /// Drops the top stack value
    OP_DROP = 117,
    /// Duplicates the top stack item
    OP_DUP = 118,
    /// Removes the second-to-top stack item
    OP_NIP = 119,
    /// Copies the second-to-top stack item to the top
    OP_OVER = 120,
    /// The item n back in the stack is copied to the top
    OP_PICK = 121,
    /// The item n back in the stack is moved to the top
    OP_ROLL = 122,
    /// The top three items on the stack are rotated to the left
    OP_ROT = 123,
    /// The top two items on the stack are swapped
    OP_SWAP = 124,
    /// The item at the top of the stack is copied and inserted before the second-to-top item
    OP_TUCK = 125,
    /// Removes the top two items from the stack
    OP_2DROP = 109,
    /// Duplicates the top two stack items
    OP_2DUP = 110,
    /// Duplicates the top three stack items
    OP_3DUP = 111,
    /// Copies the pair of items two spaces back to the front
    OP_2OVER = 112,
    /// The fifth and sixth items back are moved to the top of the stack
    OP_2ROT = 113,
    /// Swaps the top two pairs of items
    OP_2SWAP = 114,
    
    // --------------------------------------------------------------------------------------------
    // Splice
    // --------------------------------------------------------------------------------------------
    
    /// Concatenates two byte sequences
    OP_CAT = 126,
    /// Splits the byte sequence at position n
    OP_SPLIT = 127,
    /// Pushes the byte sequence length of the top stack item without popping it
    OP_SIZE = 130,
    
    // --------------------------------------------------------------------------------------------
    // Bitwise Logic
    // --------------------------------------------------------------------------------------------
    
    /// Flips all of the bits in the input
    OP_INVERT = 131,
    /// Boolean and between each bit in the inputs
    OP_AND = 132,
    /// Boolean or between each bit in the inputs
    OP_OR = 133,
    /// Boolean exclusive or between each bit in the inputs
    OP_XOR = 134,
    /// Returns 1 if the inputs are exactly equal, 0 otherwise
    OP_EQUAL = 135,
    /// Same as OP_EQUAL, but runs OP_VERIFY afterward
    OP_EQUALVERIFY = 136,
    
    // --------------------------------------------------------------------------------------------
    // Arithmetic
    // --------------------------------------------------------------------------------------------
    
    /// Adds 1 to the input
    OP_1ADD = 139,
    /// Subtracts 1 from the input
    OP_1SUB = 140,
    /// The sign of the input is flipped
    OP_NEGATE = 143,
    /// The input is made positive
    OP_ABS = 144,
    /// If the input is 0 or 1, it is flipped. Otherwise, the output will be 0.
    OP_NOT = 145,
    /// Returns 0 if the input is 0. 1 otherwise.
    OP_0NOTEQUAL = 146,
    /// Adds a to b
    OP_ADD = 147,
    /// Subtracts b from a
    OP_SUB = 148,
    /// Multiplies a by b
    OP_MUL = 149,
    /// Divides a by b
    OP_DIV = 150,
    /// Returns the remainder after dividing a by b
    OP_MOD = 151,
    /// Shifts a left b bits, preserving sign
    OP_LSHIFT = 152,
    /// Shifts a right b bits, preserving sign
    OP_RSHIFT = 153,
    /// If both a and b are not empty, the output is 1. Otherwise, 0.
    OP_BOOLAND = 154,
    /// If a or b is not empty, the output is 1. Otherwise, 0.
    OP_BOOLOR = 155,
    /// Returns 1 if the numbers are equal. Otherwise, 0.
    OP_NUMEQUAL = 156,
    /// Same as OP_NUMEQUAL, but runs OP_VERIFY afterward
    OP_NUMEQUALVERIFY = 157,
    /// Returns 1 if the numbers are not equal. Otherwise, 0.
    OP_NUMNOTEQUAL = 158,
    /// Returns 1 if a is less than b. Otherwise, 0.
    OP_LESSTHAN = 159,
    /// Returns 1 if a is greater than b. Otherwise, 0.
    OP_GREATERTHAN = 160,
    /// Returns 1 if a is less than or equal to b. Otherwise, 0.
    OP_LESSTHANOREQUAL = 161,
    /// Returns 1 if a is greater than or equal to b. Otherwise, 0.
    OP_GREATERTHANOREQUAL = 162,
    /// Returns the smaller of a and b
    OP_MIN = 163,
    /// Returns the larger of a and b
    OP_MAX = 164,
    /// Returns 1 if x is within the specified range, left inclusive. Otherwise, 0.
    OP_WITHIN = 165,
    /// Converts numeric value a into a byte sequence of length b
    OP_NUM2BIN = 128,
    /// Converts byte sequence x into a numeric value
    OP_BIN2NUM = 129,
    
    // --------------------------------------------------------------------------------------------
    // Cryptography
    // --------------------------------------------------------------------------------------------
    
    /// The input is hashed using RIPEMD-160
    OP_RIPEMD160 = 166,
    /// The input is hashed using SHA-1
    OP_SHA1 = 167,
    /// The input is hashed using SHA-256
    OP_SHA256 = 168,
    /// The input is hashed twice: first with SHA-256 and then with RIPEMD-160
    OP_HASH160 = 169,
    /// The input is hashed two times with SHA-256
    OP_HASH256 = 170,
    /// Marks the part of the script after which the signature will begin matching
    OP_CODESEPARATOR = 171,
    /// Puts 1 on the stack if the signature authorizes the public key and transaction hash. Otherwise 0.
    OP_CHECKSIG = 172,
    /// Same as OP_CHECKSIG, but OP_VERIFY is executed afterward
    OP_CHECKSIGVERIFY = 173,
    /// Puts 1 on the stack if m of n signatures authorize the public key and transaction hash. Otherwise 0.
    OP_CHECKMULTISIG = 174,
    /// Same as OP_CHECKMULTISIG, but OP_VERIFY is executed afterward
    OP_CHECKMULTISIGVERIFY = 175,
    
    // --------------------------------------------------------------------------------------------
    // Locktime
    // --------------------------------------------------------------------------------------------
    
    /// Marks transaction as invalid if the top stack item is greater than the transaction's lock_time
    OP_CHECKLOCKTIMEVERIFY = 177,
    /// Marks transaction as invalid if the top stack item is less than the transaction's sequence used for relative lock time
    OP_CHECKSEQUENCEVERIFY = 178,
    
    // --------------------------------------------------------------------------------------------
    // Pseudo-words
    // --------------------------------------------------------------------------------------------
    
    /// Represents a public key hashed with OP_HASH160
    OP_PUBKEYHASH = 253,
    /// Represents a public key compatible with OP_CHECKSIG
    OP_PUBKEY = 254,
    /// Matches any opcode that is not yet assigned
    OP_INVALIDOPCODE = 255,
    
    // --------------------------------------------------------------------------------------------
    // Reserved words
    // --------------------------------------------------------------------------------------------
    
    /// Transaction is invalid unless occuring in an unexecuted OP_IF branch
    OP_RESERVED = 80,
    /// Transaction is invalid unless occuring in an unexecuted OP_IF branch
    OP_VER = 98,
    /// Transaction is invalid even when occuring in an unexecuted OP_IF branch
    OP_VERIF = 101,
    /// Transaction is invalid even when occuring in an unexecuted OP_IF branch
    OP_VERNOTIF = 102,
    /// Transaction is invalid unless occuring in an unexecuted OP_IF branch
    OP_RESERVED1 = 137,
    /// Transaction is invalid unless occuring in an unexecuted OP_IF branch
    OP_RESERVED2 = 138,
    /// The word is ignored. Does not mark transaction as invalid.
    OP_NOP1 = 176,
    /// The word is ignored. Does not mark transaction as invalid.
    OP_NOP4 = 179,
    /// The word is ignored. Does not mark transaction as invalid.
    OP_NOP5 = 180,
    /// The word is ignored. Does not mark transaction as invalid.
    OP_NOP6 = 181,
    /// The word is ignored. Does not mark transaction as invalid.
    OP_NOP7 = 182,
    /// The word is ignored. Does not mark transaction as invalid.
    OP_NOP8 = 183,
    /// The word is ignored. Does not mark transaction as invalid.
    OP_NOP9 = 184,
    /// The word is ignored. Does not mark transaction as invalid.
    OP_NOP10 = 185,
    
    /// Words at or above this number are invalid
    OP_INVALID_ABOVE = 186,
    
    // --------------------------------------------------------------------------------------------
    // Disabled words
    // --------------------------------------------------------------------------------------------
    
    /// The input is multiplied by 2
    OP_2MUL = 141,
    /// The input is divided by 2
    OP_2DIV = 142,
}

impl Operation {
    // helper function to get pushdata of a particular size from the buffer
    fn get_pushdata(size: usize, buffer: &mut dyn Buf) -> BsvResult<Bytes> where Self: Sized {
        if size > buffer.remaining() {
            Err(BsvError::DataTooSmall)
        } else {
            Ok(buffer.copy_to_bytes(size))
        }
    }
}

impl Encodable for Operation {
    fn from_binary(buffer: &mut dyn Buf) -> BsvResult<Self> where Self: Sized {
        match buffer.has_remaining() {
            false => Err(BsvError::DataTooSmall),
            true => match buffer.get_u8() {
                0 => Ok(Operation::OP_0),
                76 => {
                    if buffer.has_remaining() {
                        let size = buffer.get_u8() as usize;
                        Ok(Operation::OP_PUSHDATA1(Self::get_pushdata(size, buffer)?))
                    } else {
                        Err(BsvError::DataTooSmall)
                    }
                },
                77 => {
                    if buffer.remaining() >= 2 {
                        let size = buffer.get_u16_le() as usize;
                        Ok(Operation::OP_PUSHDATA2(Self::get_pushdata(size, buffer)?))
                    } else {
                        Err(BsvError::DataTooSmall)
                    }
                },
                78 => {
                    if buffer.remaining() >= 4 {
                        let size = buffer.get_u32_le() as usize;
                        Ok(Operation::OP_PUSHDATA4(Self::get_pushdata(size, buffer)?))
                    } else {
                        Err(BsvError::DataTooSmall)
                    }
                },
                other => {
                    if other > 0 && other < 76 {
                        Ok(Operation::OP_PUSH(Self::get_pushdata(other as usize, buffer)?))
                    } else {
                        Err(BsvError::UnrecognizedOpCode)
                    }
                }
            }
        }
    }

    fn to_binary(&self, buffer: &mut dyn BufMut) -> BsvResult<()> {
        match buffer.has_remaining_mut() {
            false => Err(BsvError::DataTooSmall),
            true => match self {
                Operation::OP_0 => Ok(buffer.put_u8(0)),
                Operation::OP_PUSH(data) => {
                    if buffer.remaining_mut() < data.len() + 1 {
                        Err(BsvError::DataTooSmall)
                    } else {
                        buffer.put_u8(data.len() as u8);
                        Ok(buffer.put_slice(data))
                    }
                },
                Operation::OP_PUSHDATA1(data) => {
                    if buffer.remaining_mut() < data.len() + 2 {
                        Err(BsvError::DataTooSmall)
                    } else {
                        buffer.put_u8(76);
                        buffer.put_u8(data.len() as u8);
                        Ok(buffer.put_slice(data))
                    }
                },
                Operation::OP_PUSHDATA2(data) => {
                    if buffer.remaining_mut() < data.len() + 3 {
                        Err(BsvError::DataTooSmall)
                    } else {
                        buffer.put_u8(77);
                        buffer.put_u16_le(data.len() as u16);
                        Ok(buffer.put_slice(data))
                    }
                },
                Operation::OP_PUSHDATA4(data) => {
                    if buffer.remaining_mut() < data.len() + 5 {
                        Err(BsvError::DataTooSmall)
                    } else {
                        buffer.put_u8(78);
                        buffer.put_u32_le(data.len() as u32);
                        Ok(buffer.put_slice(data))
                    }
                },
                _ => todo!(),
            }
        }
    }

    fn size(&self) -> usize {
        match self {
            Operation::OP_PUSH(data) => {
                data.len() + 1
            },
            Operation::OP_PUSHDATA1(data) => {
                data.len() + 2
            },
            Operation::OP_PUSHDATA2(data) => {
                data.len() + 3
            },
            Operation::OP_PUSHDATA4(data) => {
                data.len() + 5
            },
            _ => 1
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bitcoin::Encodable;
    use crate::bitcoin::script::Operation;

    #[test]
    fn simple_reads() {
        let mut op1: &[u8] = &[0u8];
        let r = Operation::from_binary(&mut op1).unwrap();
        assert_eq!(r, Operation::OP_0);

        // op_push 4 bytes
        let mut op2: &[u8] = &[4u8, 0, 1, 2, 3];
        let r = Operation::from_binary(&mut op2).unwrap();
        assert!(matches!(r, Operation::OP_PUSH{ .. }));

        // op_pushdata1
        let mut op3: &[u8] = &[76u8, 4, 1, 2, 3, 4];
        let r = Operation::from_binary(&mut op3).unwrap();
        assert!(matches!(r, Operation::OP_PUSHDATA1{ .. }));
    }
}
