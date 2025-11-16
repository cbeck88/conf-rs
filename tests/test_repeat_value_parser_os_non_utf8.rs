#![cfg(feature = "serde")]

use conf::Conf;
use serde_json::json;
use std::ffi::OsString;

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
    #[conf(repeat, long, value_parser_os = count_bytes, serde(use_value_parser))]
    pub byte_counts: Vec<usize>,
}

#[test]
#[cfg(unix)]
fn test_non_utf8_args_with_use_value_parser() {
    use std::os::unix::ffi::OsStringExt;

    // Create non-UTF-8 OsString (invalid UTF-8 byte sequence)
    let non_utf8_arg1 = OsString::from_vec(vec![b'h', b'e', b'l', b'l', b'o', 0xFF, 0xFE]);
    let non_utf8_arg2 = OsString::from_vec(vec![b't', b'e', b's', b't', 0x80, 0x81, 0x82]);

    let result = Config::conf_builder()
        .args([
            OsString::from("test_app"),
            OsString::from("--byte-counts"),
            non_utf8_arg1,
            OsString::from("--byte-counts"),
            non_utf8_arg2,
        ])
        .try_parse()
        .unwrap();

    assert_eq!(result.byte_counts.len(), 2);
    assert_eq!(result.byte_counts[0], 7); // "hello" + 2 invalid bytes
    assert_eq!(result.byte_counts[1], 7); // "test" + 3 invalid bytes
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
            OsString::from("--byte-counts"),
            non_utf8_arg,
        ])
        .doc(
            "test_doc",
            json!({
                "byte_counts": ["hello", "world"]
            }),
        )
        .try_parse()
        .unwrap();

    // CLI should override serde, and value_parser_os should handle non-UTF-8
    assert_eq!(result.byte_counts.len(), 1);
    assert_eq!(result.byte_counts[0], 4); // "abc" + 1 invalid byte
}

#[test]
fn test_serde_with_use_value_parser_provides_valid_utf8() {
    // When serde provides data with use_value_parser, it's always valid UTF-8
    let result = Config::conf_builder()
        .args(["test_app"])
        .doc(
            "test_doc",
            json!({
                "byte_counts": ["hello", "world!", "test"]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.byte_counts.len(), 3);
    assert_eq!(result.byte_counts[0], 5); // "hello"
    assert_eq!(result.byte_counts[1], 6); // "world!"
    assert_eq!(result.byte_counts[2], 4); // "test"
}

#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct ConfigWithoutUseValueParser {
    #[conf(repeat, long, value_parser_os = count_bytes)]
    pub byte_counts: Vec<usize>,
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
            OsString::from("--byte-counts"),
            non_utf8_arg,
        ])
        .try_parse()
        .unwrap();

    assert_eq!(result.byte_counts.len(), 1);
    assert_eq!(result.byte_counts[0], 5); // "xyz" + 2 invalid bytes
}

#[test]
fn test_serde_without_use_value_parser_ignores_value_parser_os() {
    // Without use_value_parser, serde deserializes directly into Vec<usize>
    let result = ConfigWithoutUseValueParser::conf_builder()
        .args(["test_app"])
        .doc(
            "test_doc",
            json!({
                "byte_counts": [100, 200, 300]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.byte_counts.len(), 3);
    assert_eq!(result.byte_counts[0], 100); // Direct from JSON
    assert_eq!(result.byte_counts[1], 200); // Direct from JSON
    assert_eq!(result.byte_counts[2], 300); // Direct from JSON
}
