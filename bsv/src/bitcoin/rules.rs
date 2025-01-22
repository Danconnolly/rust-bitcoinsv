#![allow(non_snake_case)]
#![allow(dead_code)]     // lots of important definitions in here that may not be used

/// Bitcoin SV has a number of rules, consensus rules, and policy values. These are defined in this module.
///
/// This module makes the consensus rule and policy values available. Applications will generally need
/// the policy rule values except when dealing with transactions from confirmed blocks in which case
/// they will need the consensus rule values. The consensus rule values are more generally more permissive
/// than the policy rule values.
///
/// These values are sometimes needed deep within the code so we have chosen to make these available
/// through atomic global variables rather than having to pass a reference to configuration through
/// myriad levels of functions. It is not expected that these values will change dynamically, however
/// that capability is possible so that we can support online reconfiguration.
///
/// Rules, Consensus Rules, and Policy values are described in the [Genesis Upgrade specification](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/genesis-spec.md).
///
/// The values in this version of the module are valid for the Bitcoin SV
/// blockchains after the Genesis Upgrade.

use std::sync::atomic::{AtomicU64, Ordering};

/// Configurable Consensus Rule for Miners - maximum block size - default value 4GB
static CRULE_MAX_BLOCK_SIZE: AtomicU64 = AtomicU64::new(4_000_000_000);

/// Configurable Consensus Rule for Clients - maximum block size - default value 10GB
static POLICY_MAX_BLOCK_SIZE: AtomicU64 = AtomicU64::new(10_000_000_000);

/// Get the policy or consensus rule value of the maximum size of a block.
///
/// If `policy` is true then get the policy value, otherwise get the consensus rule value.
pub fn MAX_BLOCK_SIZE(policy: bool) -> u64 {
    if policy {
        POLICY_MAX_BLOCK_SIZE.load(Ordering::Relaxed)
    } else {
        CRULE_MAX_BLOCK_SIZE.load(Ordering::Relaxed)
    }
}

/// Consensus Rule - maximum size of transactions - 1GB - [link](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/genesis-spec.md#maximum-transaction-size)
static CRULE_MAX_TX_SIZE: AtomicU64 = AtomicU64::new(1_000_000_000);

/// Policy - maximum transaction size - default 10MB - [link](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/genesis-spec.md#maximum-acceptable-transaction-size-policy)
static POLICY_MAX_TX_SIZE: AtomicU64 = AtomicU64::new(10_000_000);

/// Get the policy or consensus rule value of the maximum size of a transaction.
///
/// If `policy` is true then get the policy value, otherwise get the consensus rule value.
pub fn MAX_TX_SIZE(policy: bool) -> u64 {
    if policy {
        POLICY_MAX_TX_SIZE.load(Ordering::Relaxed)
    } else {
        CRULE_MAX_TX_SIZE.load(Ordering::Relaxed)
    }
}

/// Consensus Rule - max size of byte sequence - UINT32_MAX
static CRULE_MAX_BYTE_SEQ_LEN: AtomicU64 = AtomicU64::new(u32::MAX as u64);

/// Policy - max size of byte sequence - UINT32_MAX - local to this software
static POLICY_MAX_BYTE_SEQ_LEN: AtomicU64 = AtomicU64::new(u32::MAX as u64);

/// Get the policy or consensus rule value of the maximum length of a byte sequence.
///
/// If `policy` is true then get the policy value, otherwise get the consensus rule value.
pub fn MAX_BYTE_SEQ_LEN(policy: bool) -> u64 {
    if policy {
        POLICY_MAX_BYTE_SEQ_LEN.load(Ordering::Relaxed)
    } else {
        CRULE_MAX_BYTE_SEQ_LEN.load(Ordering::Relaxed)
    }
}

/// Consensus Rule - max number of public keys per multisig - INT32_MAX - [link](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/genesis-spec.md#number-of-public-keys-per-multisig-consensus-rule)
static CRULE_MAX_MULTISIG_KEYS: AtomicU64 = AtomicU64::new(i32::MAX as u64);

/// Policy - max number of public keys per multisig - 32 - local to this software
static POLICY_MAX_MULTISIG_KEYS: AtomicU64 = AtomicU64::new(32);

/// Get the policy or consensus rule value of the maximum number of public keys per multisig.
///
/// If `policy` is true then get the policy value, otherwise get the consensus rule value.
pub fn MAX_MULTISIG_KEYS(policy: bool) -> u64 {
    if policy {
        POLICY_MAX_MULTISIG_KEYS.load(Ordering::Relaxed)
    } else {
        CRULE_MAX_MULTISIG_KEYS.load(Ordering::Relaxed)
    }
}

/// Configurable Consensus Rule - max memory used by stacks - default = 200MB - [link](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/genesis-spec.md#stack-memory-usage-consensus-rule)
const CRULE_MAX_STACK_MEM: AtomicU64 = AtomicU64::new(200_000_000);

/// Policy - max memory used by stacks - default = 100MB - [link](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/genesis-spec.md#stack-memory-usage-policy)
const POLICY_MAX_STACK_MEM: AtomicU64 = AtomicU64::new(100_000_000);

/// Get the policy or consensus rule value of the maximum memory used by the stacks.
///
/// If `policy` is true then get the policy value, otherwise get the consensus rule value.
pub fn MAX_STACK_MEM(policy: bool) -> u64 {
    if policy {
        POLICY_MAX_STACK_MEM.load(Ordering::Relaxed)
    } else {
        CRULE_MAX_STACK_MEM.load(Ordering::Relaxed)
    }
}

/// Policy - transaction evaluation timeout - default 1s = 1000 ms - [link](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/genesis-spec.md#transaction-evaluation-timeout)
const POLICY_TX_EVAL_TIMEOUT_MS: AtomicU64 = AtomicU64::new(1_000);

/// Get the policy value of the timeout for evaluating a transaction.
pub fn TX_EVALE_TIMEOUT_MS() -> u64 {
    POLICY_TX_EVAL_TIMEOUT_MS.load(Ordering::Relaxed)
}

/// Consensus Rule - max size of numeric value - 750_000 bytes - [link](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/genesis-spec.md#numeric-value-size-consensus-rule)
/// See also [POLICY_MAX_NUMERIC_LEN].
static CRULE_MAX_NUMERIC_LEN: AtomicU64 = AtomicU64::new(750_000);

/// Policy - max numeric value length - default 250_000 - [link](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/genesis-spec.md#numeric-value-length)
/// See also [CRULE_MAX_NUMERIC_LEN_VAL].
static POLICY_MAX_NUMERIC_LEN: AtomicU64 = AtomicU64::new(250_000);

/// Get the policy or consensus rule value of the maximum length of a numeric value.
///
/// If `policy` is true then get the policy value, otherwise get the consensus rule value.
pub fn MAX_NUMERIC_LEN(policy: bool) -> u64 {
    if policy {
        POLICY_MAX_NUMERIC_LEN.load(Ordering::Relaxed)
    } else {
        CRULE_MAX_NUMERIC_LEN.load(Ordering::Relaxed)
    }
}
