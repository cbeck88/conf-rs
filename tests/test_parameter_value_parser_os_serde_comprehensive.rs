#![cfg(feature = "serde")]

use conf::Conf;
use serde_json::json;

// A value parser that returns the length of the OsStr argument
fn osstr_len(s: &std::ffi::OsStr) -> Result<usize, String> {
    Ok(s.len())
}

#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct Config {
    #[conf(long, value_parser_os = osstr_len)]
    pub length: usize,
}

#[test]
fn test_value_parser_os_from_cli_args() {
    // When specified via CLI args, value_parser_os should count bytes
    let result = Config::conf_builder()
        .args(["test_app", "--length", "hello"])
        .try_parse()
        .unwrap();

    assert_eq!(result.length, "hello".len());
}

#[test]
fn test_value_parser_os_from_serde_without_use_value_parser() {
    // When specified via serde WITHOUT use_value_parser, serde deserializes directly
    // into usize, ignoring the value_parser_os entirely
    let result = Config::conf_builder()
        .args(["test_app"])
        .doc(
            "test_doc",
            json!({
                "length": 42
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.length, 42); // Direct from JSON
}

#[test]
fn test_value_parser_os_cli_overrides_serde_without_use_value_parser() {
    // CLI args should override serde values
    let result = Config::conf_builder()
        .args(["test_app", "--length", "abc"])
        .doc(
            "test_doc",
            json!({
                "length": 100
            }),
        )
        .try_parse()
        .unwrap();

    // CLI should override file, and value_parser_os should be used for CLI
    assert_eq!(result.length, "abc".len());
}

#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct ConfigWithUseValueParser {
    #[conf(long, value_parser_os = osstr_len, serde(use_value_parser))]
    pub length: usize,
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
                "length": "hello"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.length, "hello".len());
}

#[test]
fn test_value_parser_os_cli_overrides_serde_with_use_value_parser() {
    // CLI args should override serde values, both use value_parser_os
    let result = ConfigWithUseValueParser::conf_builder()
        .args(["test_app", "--length", "test"])
        .doc(
            "test_doc",
            json!({
                "length": "from_file"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.length, "test".len());
}
