#![cfg(feature = "serde")]

use conf::Conf;
use serde_json::json;
use std::ffi::OsString;

// A value parser that returns the length of the OsStr argument
fn osstr_len(s: &std::ffi::OsStr) -> Result<usize, String> {
    Ok(s.len())
}

#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct Config {
    #[conf(long, env, value_parser_os = osstr_len, serde(use_value_parser))]
    pub length: usize,
}

#[test]
#[cfg(unix)]
fn test_non_utf8_args_with_use_value_parser() {
    use std::os::unix::ffi::OsStringExt;

    // Create non-UTF-8 OsString (invalid UTF-8 byte sequence)
    let non_utf8_arg = OsString::from_vec(vec![b'h', b'e', b'l', b'l', b'o', 0xFF, 0xFE]);

    let result = Config::conf_builder()
        .args([
            OsString::from("test_app"),
            OsString::from("--length"),
            non_utf8_arg.clone(),
        ])
        .try_parse()
        .unwrap();

    assert_eq!(result.length, non_utf8_arg.len());
}

#[test]
#[cfg(unix)]
fn test_non_utf8_args_override_serde_with_use_value_parser() {
    use std::os::unix::ffi::OsStringExt;

    // Create non-UTF-8 OsString
    let non_utf8_arg = OsString::from_vec(vec![b'a', b'b', b'c', 0xFF]);

    let result = Config::conf_builder()
        .args([
            OsString::from("test_app"),
            OsString::from("--length"),
            non_utf8_arg.clone(),
        ])
        .doc(
            "test_doc",
            json!({
                "length": "hello"
            }),
        )
        .try_parse()
        .unwrap();

    // CLI should override serde, and value_parser_os should handle non-UTF-8
    assert_eq!(result.length, non_utf8_arg.len());
}

#[test]
fn test_serde_with_use_value_parser_provides_valid_utf8() {
    // When serde provides data with use_value_parser, it's always valid UTF-8
    let result = Config::conf_builder()
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

#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct ConfigWithoutUseValueParser {
    #[conf(long, value_parser_os = osstr_len)]
    pub length: usize,
}

#[test]
#[cfg(unix)]
fn test_non_utf8_args_without_use_value_parser() {
    use std::os::unix::ffi::OsStringExt;

    // Create non-UTF-8 OsString
    let non_utf8_arg = OsString::from_vec(vec![b'x', b'y', b'z', 0xFE, 0xFF]);

    let result = ConfigWithoutUseValueParser::conf_builder()
        .args([
            OsString::from("test_app"),
            OsString::from("--length"),
            non_utf8_arg.clone(),
        ])
        .try_parse()
        .unwrap();

    assert_eq!(result.length, non_utf8_arg.len());
}

#[test]
fn test_serde_without_use_value_parser_ignores_value_parser_os() {
    // Without use_value_parser, serde deserializes directly into usize
    let result = ConfigWithoutUseValueParser::conf_builder()
        .args(["test_app"])
        .doc(
            "test_doc",
            json!({
                "length": 100
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.length, 100); // Direct from JSON
}

#[test]
#[cfg(unix)]
fn test_non_utf8_env_with_use_value_parser() {
    use std::os::unix::ffi::OsStringExt;

    // Create non-UTF-8 OsString for env
    let non_utf8_env = OsString::from_vec(vec![b't', b'e', b's', b't', 0x80, 0x81]);

    let result = Config::conf_builder()
        .args([OsString::from("test_app")])
        .env([(OsString::from("LENGTH"), non_utf8_env.clone())])
        .try_parse()
        .unwrap();

    assert_eq!(result.length, non_utf8_env.len());
}
