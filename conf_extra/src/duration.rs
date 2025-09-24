use core::time::Duration;

#[cfg(feature = "chrono")]
use chrono::TimeDelta;

/// Parse a `std::time::Duration` from human-readable string using endings like d, h, m, s, ms, ns
/// e.g. "500ms", "4h15m", etc.
/// See docu for `go-parse-duration` crate for details on the format
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let nanos = go_parse_duration::parse_duration(s).map_err(|e| format!("{e:?}"))?;
    let nanos = u64::try_from(nanos).map_err(|e| e.to_string())?;
    Ok(Duration::from_nanos(nanos))
}

/// Parse a `chrono::TimeDelta` from a string using endings like d, h, m, s, ms, ns
/// e.g. "500ms", "4h15m", etc.
/// See docu for `go-parse-duration` crate for details on the format
#[cfg(feature = "chrono")]
pub fn parse_time_delta(s: &str) -> Result<TimeDelta, String> {
    let nanos = go_parse_duration::parse_duration(s).map_err(|e| format!("{e:?}"))?;

    let billion = 1_000_000_000;
    let secs = nanos.div_euclid(billion);
    let subsec_nanos = nanos.rem_euclid(billion) as u32;
    TimeDelta::new(secs, subsec_nanos).ok_or_else(|| "invalid time delta".to_owned())
}
