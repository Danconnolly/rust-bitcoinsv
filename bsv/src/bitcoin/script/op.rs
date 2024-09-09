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
    OP_1NEGATE,
    /// Pushes 1 onto the stack
    OP_1,
    /// Pushes 2 onto the stack
    OP_2,
    /// Pushes 3 onto the stack
    OP_3,
    /// Pushes 4 onto the stack
    OP_4,
    /// Pushes 5 onto the stack
    OP_5,
    /// Pushes 6 onto the stack
    OP_6,
    /// Pushes 7 onto the stack
    OP_7,
    /// Pushes 8 onto the stack
    OP_8,
    /// Pushes 9 onto the stack
    OP_9,
    /// Pushes 10 onto the stack
    OP_10,
    /// Pushes 11 onto the stack
    OP_11,
    /// Pushes 12 onto the stack
    OP_12,
    /// Pushes 13 onto the stack
    OP_13,
    /// Pushes 14 onto the stack
    OP_14,
    /// Pushes 15 onto the stack
    OP_15,
    /// Pushes 16 onto the stack
    OP_16,

    // --------------------------------------------------------------------------------------------
    // Flow Control
    // --------------------------------------------------------------------------------------------
    
    /// Does nothing
    OP_NOP,
    /// If the top stack is true, statements are executed. Top stack value is removed.
    OP_IF,
    /// If the top stack is false, statements are executed. Top stack value is removed.
    OP_NOTIF,
    /// If the preceding OP_IF or OP_NOTIF statemetns were not executed, then statements are executed.
    OP_ELSE,
    /// Ends an if-else block
    OP_ENDIF,
    /// Marks a statement as invalid if the top stack value is false. Top stack value is removed.
    OP_VERIFY,
    /// Marks a statements as invalid
    OP_RETURN,
    
    // --------------------------------------------------------------------------------------------
    // Stack
    // --------------------------------------------------------------------------------------------
    
    /// Moves the top item on the main stack to the alt stack
    OP_TOALTSTACK,
    /// Moves the top item on the alt stack to the main stack
    OP_FROMALTSTACK,
    /// Duplicates the top stack value if it is not zero
    OP_IFDUP,
    /// Puts the number of stack items onto the stack
    OP_DEPTH,
    /// Drops the top stack value
    OP_DROP,
    /// Duplicates the top stack item
    OP_DUP,
    /// Removes the second-to-top stack item
    OP_NIP,
    /// Copies the second-to-top stack item to the top
    OP_OVER,
    /// The item n back in the stack is copied to the top
    OP_PICK,
    /// The item n back in the stack is moved to the top
    OP_ROLL,
    /// The top three items on the stack are rotated to the left
    OP_ROT,
    /// The top two items on the stack are swapped
    OP_SWAP,
    /// The item at the top of the stack is copied and inserted before the second-to-top item
    OP_TUCK,
    /// Removes the top two items from the stack
    OP_2DROP,
    /// Duplicates the top two stack items
    OP_2DUP,
    /// Duplicates the top three stack items
    OP_3DUP,
    /// Copies the pair of items two spaces back to the front
    OP_2OVER,
    /// The fifth and sixth items back are moved to the top of the stack
    OP_2ROT,
    /// Swaps the top two pairs of items
    OP_2SWAP,
    
    // --------------------------------------------------------------------------------------------
    // Splice
    // --------------------------------------------------------------------------------------------
    
    /// Concatenates two byte sequences
    OP_CAT,
    /// Splits the byte sequence at position n
    OP_SPLIT,
    /// Pushes the byte sequence length of the top stack item without popping it
    OP_SIZE,
    
    // --------------------------------------------------------------------------------------------
    // Bitwise Logic
    // --------------------------------------------------------------------------------------------
    
    /// Flips all of the bits in the input
    OP_INVERT,
    /// Boolean and between each bit in the inputs
    OP_AND,
    /// Boolean or between each bit in the inputs
    OP_OR,
    /// Boolean exclusive or between each bit in the inputs
    OP_XOR,
    /// Returns 1 if the inputs are exactly equal, 0 otherwise
    OP_EQUAL,
    /// Same as OP_EQUAL, but runs OP_VERIFY afterward
    OP_EQUALVERIFY,
    
    // --------------------------------------------------------------------------------------------
    // Arithmetic
    // --------------------------------------------------------------------------------------------
    
    /// Adds 1 to the input
    OP_1ADD,
    /// Subtracts 1 from the input
    OP_1SUB,
    /// The input is multiplied by 2 - disabled
    OP_2MUL,
    /// The input is divided by 2 - disabled
    OP_2DIV,
    /// The sign of the input is flipped
    OP_NEGATE,
    /// The input is made positive
    OP_ABS,
    /// If the input is 0 or 1, it is flipped. Otherwise, the output will be 0.
    OP_NOT,
    /// Returns 0 if the input is 0. 1 otherwise.
    OP_0NOTEQUAL,
    /// Adds a to b
    OP_ADD,
    /// Subtracts b from a
    OP_SUB,
    /// Multiplies a by b
    OP_MUL,
    /// Divides a by b
    OP_DIV,
    /// Returns the remainder after dividing a by b
    OP_MOD,
    /// Shifts a left b bits, preserving sign
    OP_LSHIFT,
    /// Shifts a right b bits, preserving sign
    OP_RSHIFT,
    /// If both a and b are not empty, the output is 1. Otherwise, 0.
    OP_BOOLAND,
    /// If a or b is not empty, the output is 1. Otherwise, 0.
    OP_BOOLOR,
    /// Returns 1 if the numbers are equal. Otherwise, 0.
    OP_NUMEQUAL,
    /// Same as OP_NUMEQUAL, but runs OP_VERIFY afterward
    OP_NUMEQUALVERIFY,
    /// Returns 1 if the numbers are not equal. Otherwise, 0.
    OP_NUMNOTEQUAL,
    /// Returns 1 if a is less than b. Otherwise, 0.
    OP_LESSTHAN,
    /// Returns 1 if a is greater than b. Otherwise, 0.
    OP_GREATERTHAN,
    /// Returns 1 if a is less than or equal to b. Otherwise, 0.
    OP_LESSTHANOREQUAL,
    /// Returns 1 if a is greater than or equal to b. Otherwise, 0.
    OP_GREATERTHANOREQUAL,
    /// Returns the smaller of a and b
    OP_MIN,
    /// Returns the larger of a and b
    OP_MAX,
    /// Returns 1 if x is within the specified range, left inclusive. Otherwise, 0.
    OP_WITHIN,
    /// Converts numeric value a into a byte sequence of length b
    OP_NUM2BIN,
    /// Converts byte sequence x into a numeric value
    OP_BIN2NUM,
    
    // --------------------------------------------------------------------------------------------
    // Cryptography
    // --------------------------------------------------------------------------------------------
    
    /// The input is hashed using RIPEMD-160
    OP_RIPEMD160,
    /// The input is hashed using SHA-1
    OP_SHA1,
    /// The input is hashed using SHA-256
    OP_SHA256,
    /// The input is hashed twice: first with SHA-256 and then with RIPEMD-160
    OP_HASH160,
    /// The input is hashed two times with SHA-256
    OP_HASH256,
    /// Marks the part of the script after which the signature will begin matching
    OP_CODESEPARATOR,
    /// Puts 1 on the stack if the signature authorizes the public key and transaction hash. Otherwise 0.
    OP_CHECKSIG,
    /// Same as OP_CHECKSIG, but OP_VERIFY is executed afterward
    OP_CHECKSIGVERIFY,
    /// Puts 1 on the stack if m of n signatures authorize the public key and transaction hash. Otherwise 0.
    OP_CHECKMULTISIG,
    /// Same as OP_CHECKMULTISIG, but OP_VERIFY is executed afterward
    OP_CHECKMULTISIGVERIFY,
    
    // --------------------------------------------------------------------------------------------
    // Locktime
    // --------------------------------------------------------------------------------------------
    
    /// Marks transaction as invalid if the top stack item is greater than the transaction's lock_time
    /// todo: check
    OP_CHECKLOCKTIMEVERIFY,
    /// Marks transaction as invalid if the top stack item is less than the transaction's sequence used for relative lock time
    /// todo: check
    OP_CHECKSEQUENCEVERIFY,
    
    // --------------------------------------------------------------------------------------------
    // Reserved words
    // --------------------------------------------------------------------------------------------
    
    /// Transaction is invalid unless occuring in an unexecuted OP_IF branch
    OP_RESERVED,
    /// Transaction is invalid unless occuring in an unexecuted OP_IF branch
    OP_VER,
    /// Transaction is invalid even when occuring in an unexecuted OP_IF branch
    OP_VERIF,
    /// Transaction is invalid even when occuring in an unexecuted OP_IF branch
    OP_VERNOTIF,
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
                79 => Ok(Operation::OP_1NEGATE),
                80 => Ok(Operation::OP_RESERVED),
                81 => Ok(Operation::OP_1),
                82 => Ok(Operation::OP_2),
                83 => Ok(Operation::OP_3),
                84 => Ok(Operation::OP_4),
                85 => Ok(Operation::OP_5),
                86 => Ok(Operation::OP_6),
                87 => Ok(Operation::OP_7),
                88 => Ok(Operation::OP_8),
                89 => Ok(Operation::OP_9),
                90 => Ok(Operation::OP_10),
                91 => Ok(Operation::OP_11),
                92 => Ok(Operation::OP_12),
                93 => Ok(Operation::OP_13),
                94 => Ok(Operation::OP_14),
                95 => Ok(Operation::OP_15),
                96 => Ok(Operation::OP_16),
                97 => Ok(Operation::OP_NOP),
                98 => Ok(Operation::OP_VER),
                99 => Ok(Operation::OP_IF),
                100 => Ok(Operation::OP_NOTIF),
                101 => Ok(Operation::OP_VERIF),
                102 => Ok(Operation::OP_VERNOTIF),
                103 => Ok(Operation::OP_ELSE),
                104 => Ok(Operation::OP_ENDIF),
                105 => Ok(Operation::OP_VERIFY),
                106 => Ok(Operation::OP_RETURN),
                107 => Ok(Operation::OP_TOALTSTACK),
                108 => Ok(Operation::OP_FROMALTSTACK),
                109 => Ok(Operation::OP_2DROP),
                110 => Ok(Operation::OP_2DUP),
                111 => Ok(Operation::OP_3DUP),
                112 => Ok(Operation::OP_2OVER),
                113 => Ok(Operation::OP_2ROT),
                114 => Ok(Operation::OP_2SWAP),
                115 => Ok(Operation::OP_IFDUP),
                116 => Ok(Operation::OP_DEPTH),
                117 => Ok(Operation::OP_DROP),
                118 => Ok(Operation::OP_DUP),
                119 => Ok(Operation::OP_NIP),
                120 => Ok(Operation::OP_OVER),
                121 => Ok(Operation::OP_PICK),
                122 => Ok(Operation::OP_ROLL),
                123 => Ok(Operation::OP_ROT),
                124 => Ok(Operation::OP_SWAP),
                125 => Ok(Operation::OP_TUCK),
                126 => Ok(Operation::OP_CAT),
                127 => Ok(Operation::OP_SPLIT),
                128 => Ok(Operation::OP_NUM2BIN),
                129 => Ok(Operation::OP_BIN2NUM),
                130 => Ok(Operation::OP_SIZE),
                131 => Ok(Operation::OP_INVERT),
                132 => Ok(Operation::OP_AND),
                133 => Ok(Operation::OP_OR),
                134 => Ok(Operation::OP_XOR),
                135 => Ok(Operation::OP_EQUAL),
                136 => Ok(Operation::OP_EQUALVERIFY),
                137 => Ok(Operation::OP_RESERVED),
                138 => Ok(Operation::OP_RESERVED),
                139 => Ok(Operation::OP_1ADD),
                140 => Ok(Operation::OP_1SUB),
                141 => Ok(Operation::OP_2MUL),
                142 => Ok(Operation::OP_2DIV),
                143 => Ok(Operation::OP_NEGATE),
                144 => Ok(Operation::OP_ABS),
                145 => Ok(Operation::OP_NOT),
                146 => Ok(Operation::OP_0NOTEQUAL),
                147 => Ok(Operation::OP_ADD),
                148 => Ok(Operation::OP_SUB),
                149 => Ok(Operation::OP_MUL),
                150 => Ok(Operation::OP_DIV),
                151 => Ok(Operation::OP_MOD),
                152 => Ok(Operation::OP_LSHIFT),
                153 => Ok(Operation::OP_RSHIFT),
                154 => Ok(Operation::OP_BOOLAND),
                155 => Ok(Operation::OP_BOOLOR),
                156 => Ok(Operation::OP_NUMEQUAL),
                157 => Ok(Operation::OP_NUMEQUALVERIFY),
                158 => Ok(Operation::OP_NUMNOTEQUAL),
                159 => Ok(Operation::OP_LESSTHAN),
                160 => Ok(Operation::OP_GREATERTHAN),
                161 => Ok(Operation::OP_LESSTHANOREQUAL),
                162 => Ok(Operation::OP_GREATERTHANOREQUAL),
                163 => Ok(Operation::OP_MIN),
                164 => Ok(Operation::OP_MAX),
                165 => Ok(Operation::OP_WITHIN),
                166 => Ok(Operation::OP_RIPEMD160),
                167 => Ok(Operation::OP_SHA1),
                168 => Ok(Operation::OP_SHA256),
                169 => Ok(Operation::OP_HASH160),
                170 => Ok(Operation::OP_HASH256),
                171 => Ok(Operation::OP_CODESEPARATOR),
                172 => Ok(Operation::OP_CHECKSIG),
                173 => Ok(Operation::OP_CHECKSIGVERIFY),
                174 => Ok(Operation::OP_CHECKMULTISIG),
                175 => Ok(Operation::OP_CHECKMULTISIGVERIFY),
                176 => Ok(Operation::OP_NOP),
                177 => Ok(Operation::OP_CHECKLOCKTIMEVERIFY),
                178 => Ok(Operation::OP_CHECKSEQUENCEVERIFY),
                179 => Ok(Operation::OP_NOP),
                180 => Ok(Operation::OP_NOP),
                181 => Ok(Operation::OP_NOP),
                182 => Ok(Operation::OP_NOP),
                183 => Ok(Operation::OP_NOP),
                184 => Ok(Operation::OP_NOP),
                185 => Ok(Operation::OP_NOP),
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
                Operation::OP_1NEGATE => Ok(buffer.put_u8(79)),
                Operation::OP_RESERVED => Ok(buffer.put_u8(80)),
                Operation::OP_1 => Ok(buffer.put_u8(81)),
                Operation::OP_2 => Ok(buffer.put_u8(82)),
                Operation::OP_3 => Ok(buffer.put_u8(83)),
                Operation::OP_4 => Ok(buffer.put_u8(84)),
                Operation::OP_5 => Ok(buffer.put_u8(85)),
                Operation::OP_6 => Ok(buffer.put_u8(86)),
                Operation::OP_7 => Ok(buffer.put_u8(87)),
                Operation::OP_8 => Ok(buffer.put_u8(88)),
                Operation::OP_9 => Ok(buffer.put_u8(89)),
                Operation::OP_10 => Ok(buffer.put_u8(90)),
                Operation::OP_11 => Ok(buffer.put_u8(91)),
                Operation::OP_12 => Ok(buffer.put_u8(92)),
                Operation::OP_13 => Ok(buffer.put_u8(93)),
                Operation::OP_14 => Ok(buffer.put_u8(94)),
                Operation::OP_15 => Ok(buffer.put_u8(95)),
                Operation::OP_16 => Ok(buffer.put_u8(96)),
                Operation::OP_NOP => Ok(buffer.put_u8(97)),
                Operation::OP_VER => Ok(buffer.put_u8(98)),
                Operation::OP_IF => Ok(buffer.put_u8(99)),
                Operation::OP_NOTIF => Ok(buffer.put_u8(100)),
                Operation::OP_VERIF => Ok(buffer.put_u8(101)),
                Operation::OP_VERNOTIF => Ok(buffer.put_u8(102)),
                Operation::OP_ELSE => Ok(buffer.put_u8(103)),
                Operation::OP_ENDIF => Ok(buffer.put_u8(104)),
                Operation::OP_VERIFY => Ok(buffer.put_u8(105)),
                Operation::OP_RETURN => Ok(buffer.put_u8(106)),
                Operation::OP_TOALTSTACK => Ok(buffer.put_u8(107)),
                Operation::OP_FROMALTSTACK => Ok(buffer.put_u8(108)),
                Operation::OP_2DROP => Ok(buffer.put_u8(109)),
                Operation::OP_2DUP => Ok(buffer.put_u8(110)),
                Operation::OP_3DUP => Ok(buffer.put_u8(111)),
                Operation::OP_2OVER => Ok(buffer.put_u8(112)),
                Operation::OP_2ROT => Ok(buffer.put_u8(113)),
                Operation::OP_2SWAP => Ok(buffer.put_u8(114)),
                Operation::OP_IFDUP => Ok(buffer.put_u8(115)),
                Operation::OP_DEPTH => Ok(buffer.put_u8(116)),
                Operation::OP_DROP => Ok(buffer.put_u8(117)),
                Operation::OP_DUP => Ok(buffer.put_u8(118)),
                Operation::OP_NIP => Ok(buffer.put_u8(119)),
                Operation::OP_OVER => Ok(buffer.put_u8(120)),
                Operation::OP_PICK => Ok(buffer.put_u8(121)),
                Operation::OP_ROLL => Ok(buffer.put_u8(122)),
                Operation::OP_ROT => Ok(buffer.put_u8(123)),
                Operation::OP_SWAP => Ok(buffer.put_u8(124)),
                Operation::OP_TUCK => Ok(buffer.put_u8(125)),
                Operation::OP_CAT => Ok(buffer.put_u8(126)),
                Operation::OP_SPLIT => Ok(buffer.put_u8(127)),
                Operation::OP_NUM2BIN => Ok(buffer.put_u8(128)),
                Operation::OP_BIN2NUM => Ok(buffer.put_u8(129)),
                Operation::OP_SIZE => Ok(buffer.put_u8(130)),
                Operation::OP_INVERT => Ok(buffer.put_u8(131)),
                Operation::OP_AND => Ok(buffer.put_u8(132)),
                Operation::OP_OR => Ok(buffer.put_u8(133)),
                Operation::OP_XOR => Ok(buffer.put_u8(134)),
                Operation::OP_EQUAL => Ok(buffer.put_u8(135)),
                Operation::OP_EQUALVERIFY => Ok(buffer.put_u8(136)),
                Operation::OP_1ADD => Ok(buffer.put_u8(139)),
                Operation::OP_1SUB => Ok(buffer.put_u8(140)),
                Operation::OP_2MUL => Ok(buffer.put_u8(141)),
                Operation::OP_2DIV => Ok(buffer.put_u8(142)),
                Operation::OP_NEGATE => Ok(buffer.put_u8(143)),
                Operation::OP_ABS => Ok(buffer.put_u8(144)),
                Operation::OP_NOT => Ok(buffer.put_u8(145)),
                Operation::OP_0NOTEQUAL => Ok(buffer.put_u8(146)),
                Operation::OP_ADD => Ok(buffer.put_u8(147)),
                Operation::OP_SUB => Ok(buffer.put_u8(148)),
                Operation::OP_MUL => Ok(buffer.put_u8(149)),
                Operation::OP_DIV => Ok(buffer.put_u8(150)),
                Operation::OP_MOD => Ok(buffer.put_u8(151)),
                Operation::OP_LSHIFT => Ok(buffer.put_u8(152)),
                Operation::OP_RSHIFT => Ok(buffer.put_u8(153)),
                Operation::OP_BOOLAND => Ok(buffer.put_u8(154)),
                Operation::OP_BOOLOR => Ok(buffer.put_u8(155)),
                Operation::OP_NUMEQUAL => Ok(buffer.put_u8(156)),
                Operation::OP_NUMEQUALVERIFY => Ok(buffer.put_u8(157)),
                Operation::OP_NUMNOTEQUAL => Ok(buffer.put_u8(158)),
                Operation::OP_LESSTHAN => Ok(buffer.put_u8(159)),
                Operation::OP_GREATERTHAN => Ok(buffer.put_u8(160)),
                Operation::OP_LESSTHANOREQUAL => Ok(buffer.put_u8(161)),
                Operation::OP_GREATERTHANOREQUAL => Ok(buffer.put_u8(162)),
                Operation::OP_MIN => Ok(buffer.put_u8(163)),
                Operation::OP_MAX => Ok(buffer.put_u8(164)),
                Operation::OP_WITHIN => Ok(buffer.put_u8(165)),
                Operation::OP_RIPEMD160 => Ok(buffer.put_u8(166)),
                Operation::OP_SHA1 => Ok(buffer.put_u8(167)),
                Operation::OP_SHA256 => Ok(buffer.put_u8(168)),
                Operation::OP_HASH160 => Ok(buffer.put_u8(169)),
                Operation::OP_HASH256 => Ok(buffer.put_u8(170)),
                Operation::OP_CODESEPARATOR => Ok(buffer.put_u8(171)),
                Operation::OP_CHECKSIG => Ok(buffer.put_u8(172)),
                Operation::OP_CHECKSIGVERIFY => Ok(buffer.put_u8(173)),
                Operation::OP_CHECKMULTISIG => Ok(buffer.put_u8(174)),
                Operation::OP_CHECKMULTISIGVERIFY => Ok(buffer.put_u8(175)),
                Operation::OP_CHECKLOCKTIMEVERIFY => Ok(buffer.put_u8(177)),
                Operation::OP_CHECKSEQUENCEVERIFY => Ok(buffer.put_u8(178)),
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
