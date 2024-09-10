use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use crate::{BsvError, BsvResult};
use crate::bitcoin::encoding::Encodable;
use crate::bitcoin::Operation::{OP_0, OP_1, OP_10, OP_11, OP_12, OP_13, OP_14, OP_15, OP_16, OP_1NEGATE, OP_2, OP_3, OP_4, OP_5, OP_6, OP_7, OP_8, OP_9, OP_FALSE, OP_TRUE};
use crate::bitcoin::script::byte_seq::ByteSequence;

/// An Operation is an opcode plus relevant data.
///
/// todo: add Copy trait
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]      // we want to keep the Bitcoin standard naming convention
#[repr(u8)]
pub enum Operation {
    /// Pushes 0 onto the stack.
    OP_0,
    /// Pushes 0 onto the stack, alias for OP_0.
    OP_FALSE,
    /// Pushes data onto the stack where the data must be 1-75 bytes long.
    OP_PUSH(ByteSequence),
    /// The next byte sets the number of bytes to push onto the stack
    OP_PUSHDATA1(ByteSequence),
    /// The next two bytes sets the number of bytes to push onto the stack
    OP_PUSHDATA2(ByteSequence),
    /// The next four bytes sets the number of bytes to push onto the stack
    OP_PUSHDATA4(ByteSequence),
    /// Pushes -1 onto the stack
    OP_1NEGATE,
    /// Pushes 1 onto the stack
    OP_1,
    /// Pushes 1 onto the stack, alias for OP_1.
    OP_TRUE,
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
    // Reserved words
    // --------------------------------------------------------------------------------------------

    /// Upgradeable NOP. Acts as a NOP but its usage is not recommended as the codes may be redefined in
    /// the future. Policy usually rejects transactions that use this code.
    OP_UPNOP,
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

    /// Equality implementation with support for aliases.
    ///
    /// We need this because OP_0 and OP_FALSE are equal, as is OP_1 and OP_TRUE.
    pub fn eq_alias(&self, other: &Self) -> bool {
        match self {
            Operation::OP_0 | Operation::OP_FALSE => {
                match other {
                    Operation::OP_0 | Operation::OP_FALSE=> true,
                    _ => false,
                }
            },
            Operation::OP_1 | Operation::OP_TRUE=> {
                match other {
                    Operation::OP_1 | Operation::OP_TRUE => true,
                    _ => false,
                }
            },
            value => {
                other == value
            },
        }
    }

    /// Returns true if the operation pushes data on the stack.
    pub fn is_data_push(&self) -> bool {
        use Operation::*;
        match self {
            OP_0 | OP_1 | OP_2 | OP_3 | OP_4 | OP_5 | OP_6 | OP_7 | OP_8 | OP_9 | OP_10 |
            OP_11 | OP_12 | OP_13 | OP_14 | OP_15 | OP_16 | OP_FALSE | OP_TRUE | OP_1NEGATE |
            OP_PUSH(_) | OP_PUSHDATA1(_) | OP_PUSHDATA2(_) | OP_PUSHDATA4(_) => true,
            _ => false
        }
    }

    /// Returns the data pushed to stack for pushdata operations, NONE for operations that do not
    /// directly push a value to stack.
    pub fn data_pushed(&self) -> Option<Bytes> {
        use Operation::*;
        match self {
            OP_0 | OP_FALSE => Some(Bytes::from(&[0u8][..])),
            OP_1 | OP_TRUE => Some(Bytes::from(&[1u8][..])),
            OP_2 => Some(Bytes::from(&[2u8][..])),
            OP_3 => Some(Bytes::from(&[3u8][..])),
            OP_4 => Some(Bytes::from(&[4u8][..])),
            OP_5 => Some(Bytes::from(&[5u8][..])),
            OP_6 => Some(Bytes::from(&[6u8][..])),
            OP_7 => Some(Bytes::from(&[7u8][..])),
            OP_8 => Some(Bytes::from(&[8u8][..])),
            OP_9 => Some(Bytes::from(&[9u8][..])),
            OP_10 => Some(Bytes::from(&[10u8][..])),
            OP_11 => Some(Bytes::from(&[11u8][..])),
            OP_12 => Some(Bytes::from(&[12u8][..])),
            OP_13 => Some(Bytes::from(&[13u8][..])),
            OP_14 => Some(Bytes::from(&[14u8][..])),
            OP_15 => Some(Bytes::from(&[15u8][..])),
            OP_16 => Some(Bytes::from(&[16u8][..])),
            OP_1NEGATE => Some(Bytes::from(&[255u8][..])),
            OP_PUSH(data) | OP_PUSHDATA1(data) | OP_PUSHDATA2(data) | OP_PUSHDATA4(data) => Some(data.get_bytes()),
            _ => None
        }
    }

    /// Returns the number pushed to stack for pushdata operations as an i64.
    ///
    /// If the operation does not push a value to the stack or the value  is too large to be represented
    /// by an i64 then NONE is returned.
    ///
    /// In comparison to the size of numbers supported by the Bitcoin rules, an i64 is small.
    /// See [rules::MAX_NUMERIC_LEN].
    pub fn small_num_pushed(&self) -> Option<i64> {
        use Operation::*;
        match self {
            OP_0 | OP_FALSE => Some(0),
            OP_1 | OP_TRUE => Some(1),
            OP_2 => Some(2),
            OP_3 => Some(3),
            OP_4 => Some(4),
            OP_5 => Some(5),
            OP_6 => Some(6),
            OP_7 => Some(7),
            OP_8 => Some(8),
            OP_9 => Some(9),
            OP_10 => Some(10),
            OP_11 => Some(11),
            OP_12 => Some(12),
            OP_13 => Some(13),
            OP_14 => Some(14),
            OP_15 => Some(15),
            OP_16 => Some(16),
            OP_1NEGATE => Some(-1),
            OP_PUSH(data) | OP_PUSHDATA1(data) | OP_PUSHDATA2(data) | OP_PUSHDATA4(data) => {
                match data.to_small_number() {
                    Err(_) => None,
                    Ok(val) => Some(val),
                }
            },
            _ => None
        }
    }
}

impl Encodable for Operation {
    fn from_binary(buffer: &mut dyn Buf) -> BsvResult<Self> where Self: Sized {
        use Operation::*;
        match buffer.has_remaining() {
            false => Err(BsvError::DataTooSmall),
            true => match buffer.get_u8() {
                0 => Ok(OP_0),
                76 => {
                    if buffer.has_remaining() {
                        let size = buffer.get_u8() as usize;
                        Ok(OP_PUSHDATA1(ByteSequence::new(Self::get_pushdata(size, buffer)?)))
                    } else {
                        Err(BsvError::DataTooSmall)
                    }
                },
                77 => {
                    if buffer.remaining() >= 2 {
                        let size = buffer.get_u16_le() as usize;
                        Ok(OP_PUSHDATA2(ByteSequence::new(Self::get_pushdata(size, buffer)?)))
                    } else {
                        Err(BsvError::DataTooSmall)
                    }
                },
                78 => {
                    if buffer.remaining() >= 4 {
                        let size = buffer.get_u32_le() as usize;
                        Ok(OP_PUSHDATA4(ByteSequence::new(Self::get_pushdata(size, buffer)?)))
                    } else {
                        Err(BsvError::DataTooSmall)
                    }
                },
                79 => Ok(OP_1NEGATE),
                80 => Ok(OP_RESERVED),
                81 => Ok(OP_1),
                82 => Ok(OP_2),
                83 => Ok(OP_3),
                84 => Ok(OP_4),
                85 => Ok(OP_5),
                86 => Ok(OP_6),
                87 => Ok(OP_7),
                88 => Ok(OP_8),
                89 => Ok(OP_9),
                90 => Ok(OP_10),
                91 => Ok(OP_11),
                92 => Ok(OP_12),
                93 => Ok(OP_13),
                94 => Ok(OP_14),
                95 => Ok(OP_15),
                96 => Ok(OP_16),
                97 => Ok(OP_NOP),
                98 => Ok(OP_VER),
                99 => Ok(OP_IF),
                100 => Ok(OP_NOTIF),
                101 => Ok(OP_VERIF),
                102 => Ok(OP_VERNOTIF),
                103 => Ok(OP_ELSE),
                104 => Ok(OP_ENDIF),
                105 => Ok(OP_VERIFY),
                106 => Ok(OP_RETURN),
                107 => Ok(OP_TOALTSTACK),
                108 => Ok(OP_FROMALTSTACK),
                109 => Ok(OP_2DROP),
                110 => Ok(OP_2DUP),
                111 => Ok(OP_3DUP),
                112 => Ok(OP_2OVER),
                113 => Ok(OP_2ROT),
                114 => Ok(OP_2SWAP),
                115 => Ok(OP_IFDUP),
                116 => Ok(OP_DEPTH),
                117 => Ok(OP_DROP),
                118 => Ok(OP_DUP),
                119 => Ok(OP_NIP),
                120 => Ok(OP_OVER),
                121 => Ok(OP_PICK),
                122 => Ok(OP_ROLL),
                123 => Ok(OP_ROT),
                124 => Ok(OP_SWAP),
                125 => Ok(OP_TUCK),
                126 => Ok(OP_CAT),
                127 => Ok(OP_SPLIT),
                128 => Ok(OP_NUM2BIN),
                129 => Ok(OP_BIN2NUM),
                130 => Ok(OP_SIZE),
                131 => Ok(OP_INVERT),
                132 => Ok(OP_AND),
                133 => Ok(OP_OR),
                134 => Ok(OP_XOR),
                135 => Ok(OP_EQUAL),
                136 => Ok(OP_EQUALVERIFY),
                137 => Ok(OP_RESERVED),
                138 => Ok(OP_RESERVED),
                139 => Ok(OP_1ADD),
                140 => Ok(OP_1SUB),
                141 => Ok(OP_2MUL),
                142 => Ok(OP_2DIV),
                143 => Ok(OP_NEGATE),
                144 => Ok(OP_ABS),
                145 => Ok(OP_NOT),
                146 => Ok(OP_0NOTEQUAL),
                147 => Ok(OP_ADD),
                148 => Ok(OP_SUB),
                149 => Ok(OP_MUL),
                150 => Ok(OP_DIV),
                151 => Ok(OP_MOD),
                152 => Ok(OP_LSHIFT),
                153 => Ok(OP_RSHIFT),
                154 => Ok(OP_BOOLAND),
                155 => Ok(OP_BOOLOR),
                156 => Ok(OP_NUMEQUAL),
                157 => Ok(OP_NUMEQUALVERIFY),
                158 => Ok(OP_NUMNOTEQUAL),
                159 => Ok(OP_LESSTHAN),
                160 => Ok(OP_GREATERTHAN),
                161 => Ok(OP_LESSTHANOREQUAL),
                162 => Ok(OP_GREATERTHANOREQUAL),
                163 => Ok(OP_MIN),
                164 => Ok(OP_MAX),
                165 => Ok(OP_WITHIN),
                166 => Ok(OP_RIPEMD160),
                167 => Ok(OP_SHA1),
                168 => Ok(OP_SHA256),
                169 => Ok(OP_HASH160),
                170 => Ok(OP_HASH256),
                171 => Ok(OP_CODESEPARATOR),
                172 => Ok(OP_CHECKSIG),
                173 => Ok(OP_CHECKSIGVERIFY),
                174 => Ok(OP_CHECKMULTISIG),
                175 => Ok(OP_CHECKMULTISIGVERIFY),
                176 => Ok(OP_NOP),
                177 => Ok(OP_UPNOP),
                178 => Ok(OP_UPNOP),
                179 => Ok(OP_NOP),
                180 => Ok(OP_NOP),
                181 => Ok(OP_NOP),
                182 => Ok(OP_NOP),
                183 => Ok(OP_NOP),
                184 => Ok(OP_NOP),
                185 => Ok(OP_NOP),
                other => {
                    if other > 0 && other < 76 {
                        Ok(OP_PUSH(ByteSequence::new(Self::get_pushdata(other as usize, buffer)?)))
                    } else {
                        Err(BsvError::UnrecognizedOpCode)
                    }
                }
            }
        }
    }

    fn to_binary(&self, buffer: &mut dyn BufMut) -> BsvResult<()> {
        use Operation::*;
        match buffer.has_remaining_mut() {
            false => Err(BsvError::DataTooSmall),
            true => match self {
                OP_0 => Ok(buffer.put_u8(0)),
                OP_FALSE => Ok(buffer.put_u8(0)),
                OP_PUSH(data) => {
                    if buffer.remaining_mut() < data.len() + 1 {
                        Err(BsvError::DataTooSmall)
                    } else {
                        buffer.put_u8(data.len() as u8);
                        Ok(buffer.put_slice(&*data.get_bytes()))
                    }
                },
                OP_PUSHDATA1(data) => {
                    if buffer.remaining_mut() < data.len() + 2 {
                        Err(BsvError::DataTooSmall)
                    } else {
                        buffer.put_u8(76);
                        buffer.put_u8(data.len() as u8);
                        Ok(buffer.put_slice(&*data.get_bytes()))
                    }
                },
                OP_PUSHDATA2(data) => {
                    if buffer.remaining_mut() < data.len() + 3 {
                        Err(BsvError::DataTooSmall)
                    } else {
                        buffer.put_u8(77);
                        buffer.put_u16_le(data.len() as u16);
                        Ok(buffer.put_slice(&*data.get_bytes()))
                    }
                },
                OP_PUSHDATA4(data) => {
                    if buffer.remaining_mut() < data.len() + 5 {
                        Err(BsvError::DataTooSmall)
                    } else {
                        buffer.put_u8(78);
                        buffer.put_u32_le(data.len() as u32);
                        Ok(buffer.put_slice(&*data.get_bytes()))
                    }
                },
                OP_1NEGATE => Ok(buffer.put_u8(79)),
                OP_RESERVED => Ok(buffer.put_u8(80)),
                OP_1 => Ok(buffer.put_u8(81)),
                OP_TRUE => Ok(buffer.put_u8(81)),
                OP_2 => Ok(buffer.put_u8(82)),
                OP_3 => Ok(buffer.put_u8(83)),
                OP_4 => Ok(buffer.put_u8(84)),
                OP_5 => Ok(buffer.put_u8(85)),
                OP_6 => Ok(buffer.put_u8(86)),
                OP_7 => Ok(buffer.put_u8(87)),
                OP_8 => Ok(buffer.put_u8(88)),
                OP_9 => Ok(buffer.put_u8(89)),
                OP_10 => Ok(buffer.put_u8(90)),
                OP_11 => Ok(buffer.put_u8(91)),
                OP_12 => Ok(buffer.put_u8(92)),
                OP_13 => Ok(buffer.put_u8(93)),
                OP_14 => Ok(buffer.put_u8(94)),
                OP_15 => Ok(buffer.put_u8(95)),
                OP_16 => Ok(buffer.put_u8(96)),
                OP_NOP => Ok(buffer.put_u8(97)),
                OP_VER => Ok(buffer.put_u8(98)),
                OP_IF => Ok(buffer.put_u8(99)),
                OP_NOTIF => Ok(buffer.put_u8(100)),
                OP_VERIF => Ok(buffer.put_u8(101)),
                OP_VERNOTIF => Ok(buffer.put_u8(102)),
                OP_ELSE => Ok(buffer.put_u8(103)),
                OP_ENDIF => Ok(buffer.put_u8(104)),
                OP_VERIFY => Ok(buffer.put_u8(105)),
                OP_RETURN => Ok(buffer.put_u8(106)),
                OP_TOALTSTACK => Ok(buffer.put_u8(107)),
                OP_FROMALTSTACK => Ok(buffer.put_u8(108)),
                OP_2DROP => Ok(buffer.put_u8(109)),
                OP_2DUP => Ok(buffer.put_u8(110)),
                OP_3DUP => Ok(buffer.put_u8(111)),
                OP_2OVER => Ok(buffer.put_u8(112)),
                OP_2ROT => Ok(buffer.put_u8(113)),
                OP_2SWAP => Ok(buffer.put_u8(114)),
                OP_IFDUP => Ok(buffer.put_u8(115)),
                OP_DEPTH => Ok(buffer.put_u8(116)),
                OP_DROP => Ok(buffer.put_u8(117)),
                OP_DUP => Ok(buffer.put_u8(118)),
                OP_NIP => Ok(buffer.put_u8(119)),
                OP_OVER => Ok(buffer.put_u8(120)),
                OP_PICK => Ok(buffer.put_u8(121)),
                OP_ROLL => Ok(buffer.put_u8(122)),
                OP_ROT => Ok(buffer.put_u8(123)),
                OP_SWAP => Ok(buffer.put_u8(124)),
                OP_TUCK => Ok(buffer.put_u8(125)),
                OP_CAT => Ok(buffer.put_u8(126)),
                OP_SPLIT => Ok(buffer.put_u8(127)),
                OP_NUM2BIN => Ok(buffer.put_u8(128)),
                OP_BIN2NUM => Ok(buffer.put_u8(129)),
                OP_SIZE => Ok(buffer.put_u8(130)),
                OP_INVERT => Ok(buffer.put_u8(131)),
                OP_AND => Ok(buffer.put_u8(132)),
                OP_OR => Ok(buffer.put_u8(133)),
                OP_XOR => Ok(buffer.put_u8(134)),
                OP_EQUAL => Ok(buffer.put_u8(135)),
                OP_EQUALVERIFY => Ok(buffer.put_u8(136)),
                OP_1ADD => Ok(buffer.put_u8(139)),
                OP_1SUB => Ok(buffer.put_u8(140)),
                OP_2MUL => Ok(buffer.put_u8(141)),
                OP_2DIV => Ok(buffer.put_u8(142)),
                OP_NEGATE => Ok(buffer.put_u8(143)),
                OP_ABS => Ok(buffer.put_u8(144)),
                OP_NOT => Ok(buffer.put_u8(145)),
                OP_0NOTEQUAL => Ok(buffer.put_u8(146)),
                OP_ADD => Ok(buffer.put_u8(147)),
                OP_SUB => Ok(buffer.put_u8(148)),
                OP_MUL => Ok(buffer.put_u8(149)),
                OP_DIV => Ok(buffer.put_u8(150)),
                OP_MOD => Ok(buffer.put_u8(151)),
                OP_LSHIFT => Ok(buffer.put_u8(152)),
                OP_RSHIFT => Ok(buffer.put_u8(153)),
                OP_BOOLAND => Ok(buffer.put_u8(154)),
                OP_BOOLOR => Ok(buffer.put_u8(155)),
                OP_NUMEQUAL => Ok(buffer.put_u8(156)),
                OP_NUMEQUALVERIFY => Ok(buffer.put_u8(157)),
                OP_NUMNOTEQUAL => Ok(buffer.put_u8(158)),
                OP_LESSTHAN => Ok(buffer.put_u8(159)),
                OP_GREATERTHAN => Ok(buffer.put_u8(160)),
                OP_LESSTHANOREQUAL => Ok(buffer.put_u8(161)),
                OP_GREATERTHANOREQUAL => Ok(buffer.put_u8(162)),
                OP_MIN => Ok(buffer.put_u8(163)),
                OP_MAX => Ok(buffer.put_u8(164)),
                OP_WITHIN => Ok(buffer.put_u8(165)),
                OP_RIPEMD160 => Ok(buffer.put_u8(166)),
                OP_SHA1 => Ok(buffer.put_u8(167)),
                OP_SHA256 => Ok(buffer.put_u8(168)),
                OP_HASH160 => Ok(buffer.put_u8(169)),
                OP_HASH256 => Ok(buffer.put_u8(170)),
                OP_CODESEPARATOR => Ok(buffer.put_u8(171)),
                OP_CHECKSIG => Ok(buffer.put_u8(172)),
                OP_CHECKSIGVERIFY => Ok(buffer.put_u8(173)),
                OP_CHECKMULTISIG => Ok(buffer.put_u8(174)),
                OP_CHECKMULTISIGVERIFY => Ok(buffer.put_u8(175)),
                OP_UPNOP => Ok(buffer.put_u8(177)),
            }
        }
    }

    fn size(&self) -> usize {
        use Operation::*;
        match self {
            OP_PUSH(data) => {
                data.len() + 1
            },
            OP_PUSHDATA1(data) => {
                data.len() + 2
            },
            OP_PUSHDATA2(data) => {
                data.len() + 3
            },
            OP_PUSHDATA4(data) => {
                data.len() + 5
            },
            _ => 1
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use crate::bitcoin::Encodable;
    use crate::bitcoin::script::Operation;

    /// Do a few simple read tests.
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

    /// Check that every opcode encodes and decodes to the same value.
    #[test]
    fn check_op_coding() {
        for j in 0u8..179 {
            let mut i: &[u8] = &[j];
            let o = Operation::from_binary(&mut i);
            if o.is_ok() {
                let o = o.unwrap();
                if o != Operation::OP_RESERVED && o != Operation::OP_NOP && o != Operation::OP_UPNOP {
                    let mut b = BytesMut::with_capacity(10);
                    o.to_binary(&mut b).unwrap();
                    assert_eq!(b[0], j);
                }
            } else {
                // the data ops will not parse properly without making some fake data
                // but the rest should succeed
                if j < 1 || j > 78 {
                    assert!(false);
                }
            }
        }
    }

    /// OP_0 and OP_FALSE are the same thing, same for OP_1 and OP_TRUE
    #[test]
    fn test_equality() {
        assert!(Operation::OP_FALSE.eq_alias(&Operation::OP_0));
        assert!(Operation::OP_TRUE.eq_alias(&Operation::OP_1));
    }
}
