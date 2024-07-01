use std::collections::BTreeMap;
use std::ffi::OsString;

pub type ParsedEnv = BTreeMap<String, OsString>;

/// Parse a generic thing that looks like std::env::vars_os but might be test data,
/// and store it in a searchable container.
pub fn parse_env<K, V>(env_vars_os: impl IntoIterator<Item = (K, V)>) -> ParsedEnv
where
    K: Into<OsString> + Clone,
    V: Into<OsString> + Clone,
{
    // Drop any non-utf8 env keys, since there's no way the parser can read them anyways, since we
    // don't give the user a way to specify a non-utf8 env value that should be read.
    // If some values are non-utf8, that's also going to fail if they are read, but it's possible our program
    // doesn't actually need to read those, so let's fail at the time it actually reads them instead.
    env_vars_os
        .into_iter()
        .filter_map(|(into_key, into_val)| {
            if let Ok(key) = into_key.into().into_string() {
                Some((key, into_val.into()))
            } else {
                None
            }
        })
        .collect()
}
