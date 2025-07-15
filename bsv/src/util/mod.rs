mod amount;

pub use amount::Amount;
use std::time::{SystemTime, UNIX_EPOCH};

/// Gets the time in seconds since UNIX_EPOCH, as an i64.
pub fn epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time should be after UNIX_EPOCH")
        .as_secs() as i64
}

/// Gets the time in seconds since UNIX_EPOCH, as an u32.
pub fn epoch_secs_u32() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time should be after UNIX_EPOCH")
        .as_secs() as u32
}

/// Gets the time in milli-seconds since UNIX_EPOCH.
pub fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time should be after UNIX_EPOCH")
        .as_millis() as u64
}
