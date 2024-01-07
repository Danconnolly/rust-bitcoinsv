mod amount;

use std::time::{SystemTime, UNIX_EPOCH};
pub use amount::Amount;


/// Gets the time in seconds since UNIX_EPOCH
/// Note that the Bitcoin protocol uses both i64 and u32 so we provide both
pub fn epoch_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

pub fn epoch_secs_u32() -> u32 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u32
}
