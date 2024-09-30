use clap_lex::RawArgs;
use std::ffi::OsString;

/// In some cases, you may want to grab a config file path from CLI args before doing the
/// main parse, so that you can load the config file content and pass it to conf::conf_builder(),
/// if the config file path was present.
///
/// This helper function uses `clap_lex` to do that, which is already indirectly a dependency of
/// `conf` anyways. It is only meant to be used for a one-off like this that is needed before the
/// main parse.
///
/// See `examples/serde/basic.rs` for example usage.
///
/// Limitations:
///
/// * Only returns the first example that it finds on the command line. Don't use this with a multi
///   option, only with a parameter.
/// * No error reporting. If the parameter is found but has no argument, we just return `None`. That
///   is expected to be caught in the `Conf::parse` step.
/// * No auto-generated docs.
///
/// If you use this, you should add a dummy parameter of the same name to your top-level config
/// struct, even if it is not used later in the program, so that you don't get an "unexpected
/// argument" error during the main parse, and so that this parameter will appear in the `--help`.
pub fn find_parameter(
    name: &str,
    args_os: impl IntoIterator<Item: Into<OsString>>,
) -> Option<OsString> {
    let raw = RawArgs::new(args_os);
    let mut cursor = raw.cursor();
    raw.next(&mut cursor); // Skip the bin

    while let Some(arg) = raw.next(&mut cursor) {
        if arg.is_escape() {
            return None;
        } else if let Some((maybe_flag, maybe_value)) = arg.to_long() {
            let Ok(flag) = maybe_flag else {
                continue;
            };

            if flag != name {
                continue;
            }

            // We found what we are searching for, now the two possibilities are
            // --flag=value
            // and
            // --flag value
            if let Some(value) = maybe_value {
                return Some(value.to_owned());
            } else {
                // Note: If there is no next value, it should be thought of as an error,
                // but we just return None, and let the main parse do the error reporting.
                return raw.next_os(&mut cursor).map(|os_str| os_str.to_owned());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_parameter() {
        assert_eq!(None, find_parameter("foo", ["."]));

        assert_eq!(
            "x",
            find_parameter("foo", [".", "--foo=x"])
                .unwrap()
                .to_string_lossy()
        );
        assert_eq!(None, find_parameter("fo", [".", "--foo=x"]));
        assert_eq!(None, find_parameter("fooo", [".", "--foo=x"]));
        assert_eq!(
            "x",
            find_parameter("foo", [".", "-zed", "z", "--foo=x"])
                .unwrap()
                .to_string_lossy()
        );
        assert_eq!(None, find_parameter("fo", [".", "-zed", "z", "--foo=x"]));
        assert_eq!(None, find_parameter("fooo", [".", "-zed", "z", "--foo=x"]));

        assert_eq!(
            "x",
            find_parameter("foo", [".", "--foo", "x"])
                .unwrap()
                .to_string_lossy()
        );
        assert_eq!(None, find_parameter("fo", [".", "--foo", "x"]));
        assert_eq!(None, find_parameter("fooo", [".", "--foo", "x"]));
        assert_eq!(
            "x",
            find_parameter("foo", [".", "-zed", "z", "--foo", "x"])
                .unwrap()
                .to_string_lossy()
        );
        assert_eq!(None, find_parameter("fo", [".", "-zed", "z", "--foo", "x"]));
        assert_eq!(
            None,
            find_parameter("fooo", [".", "-zed", "z", "--foo", "x"])
        );

        assert_eq!(None, find_parameter("foo", [".", "--foo"]));
        assert_eq!(None, find_parameter("foo", [".", "--bar=9", "--foo"]));

        assert_eq!(
            "x",
            find_parameter("foo", [".", "--foo=x", "--foo", "y"])
                .unwrap()
                .to_string_lossy()
        );
        assert_eq!(
            "x",
            find_parameter("foo", [".", "--foo", "x", "--foo", "y"])
                .unwrap()
                .to_string_lossy()
        );
        assert_eq!(
            "x",
            find_parameter("foo", [".", "--bar=9", "--foo=x", "--foo", "y"])
                .unwrap()
                .to_string_lossy()
        );
        assert_eq!(
            "x",
            find_parameter("foo", [".", "--bar=9", "--foo", "x", "--foo", "y"])
                .unwrap()
                .to_string_lossy()
        );

        assert_eq!(
            None,
            find_parameter("fo", [".", "--bar=9", "--foo=x", "--foo", "y"])
        );
        assert_eq!(
            None,
            find_parameter("fo", [".", "--bar=9", "--foo", "x", "--foo", "y"])
        );
    }
}
