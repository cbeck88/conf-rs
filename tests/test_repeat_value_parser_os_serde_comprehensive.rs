#![cfg(feature = "serde")]

use conf::Conf;
use serde_json::json;

// A value parser that counts the number of bytes in the OsStr argument
fn count_bytes(s: &std::ffi::OsStr) -> Result<usize, String> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(s.as_bytes().len())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        Ok(s.encode_wide().count() * 2) // Each wide char is 2 bytes
    }
}

#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct Config {
    #[conf(repeat, long, value_parser_os = count_bytes)]
    pub byte_counts: Vec<usize>,
}

#[test]
fn test_value_parser_os_from_cli_args() {
    // When specified via CLI args, value_parser_os should count bytes
    let result = Config::conf_builder()
        .args([
            "test_app",
            "--byte-counts",
            "hello",
            "--byte-counts",
            "world!",
        ])
        .try_parse()
        .unwrap();

    assert_eq!(result.byte_counts.len(), 2);
    assert_eq!(result.byte_counts[0], 5); // "hello" has 5 bytes
    assert_eq!(result.byte_counts[1], 6); // "world!" has 6 bytes
}

#[test]
fn test_value_parser_os_from_serde_without_use_value_parser() {
    // When specified via serde WITHOUT use_value_parser, serde deserializes directly
    // into Vec<usize>, ignoring the value_parser_os entirely
    let result = Config::conf_builder()
        .args(["test_app"])
        .doc(
            "test_doc",
            json!({
                "byte_counts": [10, 20, 30]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.byte_counts.len(), 3);
    assert_eq!(result.byte_counts[0], 10); // Direct from JSON
    assert_eq!(result.byte_counts[1], 20); // Direct from JSON
    assert_eq!(result.byte_counts[2], 30); // Direct from JSON
}

#[test]
fn test_value_parser_os_cli_overrides_serde_without_use_value_parser() {
    // CLI args should override serde values
    let result = Config::conf_builder()
        .args(["test_app", "--byte-counts", "abc"])
        .doc(
            "test_doc",
            json!({
                "byte_counts": [100, 200]
            }),
        )
        .try_parse()
        .unwrap();

    // CLI should override file, and value_parser_os should be used for CLI
    assert_eq!(result.byte_counts.len(), 1);
    assert_eq!(result.byte_counts[0], 3); // "abc" has 3 bytes
}

#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct ConfigWithUseValueParser {
    #[conf(repeat, long, value_parser_os = count_bytes, serde(use_value_parser))]
    pub byte_counts: Vec<usize>,
}

#[test]
fn test_value_parser_os_from_serde_with_use_value_parser() {
    // When specified via serde WITH use_value_parser, serde provides strings
    // which are converted to OsStr and passed through value_parser_os
    let result = ConfigWithUseValueParser::conf_builder()
        .args(["test_app"])
        .doc(
            "test_doc",
            json!({
                "byte_counts": ["hello", "world!"]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.byte_counts.len(), 2);
    assert_eq!(result.byte_counts[0], 5); // "hello" has 5 bytes
    assert_eq!(result.byte_counts[1], 6); // "world!" has 6 bytes
}

#[test]
fn test_value_parser_os_cli_overrides_serde_with_use_value_parser() {
    // CLI args should override serde values, both use value_parser_os
    let result = ConfigWithUseValueParser::conf_builder()
        .args(["test_app", "--byte-counts", "test"])
        .doc(
            "test_doc",
            json!({
                "byte_counts": ["from_file"]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.byte_counts.len(), 1);
    assert_eq!(result.byte_counts[0], 4); // "test" has 4 bytes
}
