/// The bsv.base module contains the base types and configuration for Bitcoin SV.

mod params;
mod var_int;


pub use self::params::Blockchain;
pub use self::var_int::VarInt;
