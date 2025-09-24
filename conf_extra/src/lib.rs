//! Extra types and value parsers for use with conf

mod duration;
pub use duration::{parse_duration, parse_time_delta};

mod password;
pub use password::Password;
