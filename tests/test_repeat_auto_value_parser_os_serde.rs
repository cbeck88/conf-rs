#![cfg(feature = "serde")]

use conf::Conf;
use serde_json::json;
use std::ffi::OsString;
use std::path::PathBuf;

// Test Vec<PathBuf> with serde but WITHOUT use_value_parser
// This should deserialize directly from serde and ignore the auto-detected value_parser_os
#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct ConfigPathBufNoUseValueParser {
    #[conf(repeat, long)]
    pub paths: Vec<PathBuf>,
}

#[test]
fn test_pathbuf_serde_without_use_value_parser() {
    // Serde should deserialize strings directly into PathBuf
    let result = ConfigPathBufNoUseValueParser::conf_builder()
        .args(["test_app"])
        .doc(
            "test_doc",
            json!({
                "paths": ["/foo/bar", "/baz/qux"]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.paths.len(), 2);
    assert_eq!(result.paths[0], PathBuf::from("/foo/bar"));
    assert_eq!(result.paths[1], PathBuf::from("/baz/qux"));
}

#[test]
fn test_pathbuf_cli_overrides_serde_without_use_value_parser() {
    let result = ConfigPathBufNoUseValueParser::conf_builder()
        .args(["test_app", "--paths", "/from/cli"])
        .doc(
            "test_doc",
            json!({
                "paths": ["/from/file"]
            }),
        )
        .try_parse()
        .unwrap();

    // CLI should override file
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], PathBuf::from("/from/cli"));
}

// Test Vec<PathBuf> with serde WITH use_value_parser
// This should use the auto-detected value_parser_os for both CLI and serde
#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct ConfigPathBufWithUseValueParser {
    #[conf(repeat, long, serde(use_value_parser))]
    pub paths: Vec<PathBuf>,
}

#[test]
fn test_pathbuf_serde_with_use_value_parser() {
    // Serde provides strings, which are converted to OsStr and passed through value_parser_os
    let result = ConfigPathBufWithUseValueParser::conf_builder()
        .args(["test_app"])
        .doc(
            "test_doc",
            json!({
                "paths": ["/foo/bar", "/baz/qux"]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.paths.len(), 2);
    assert_eq!(result.paths[0], PathBuf::from("/foo/bar"));
    assert_eq!(result.paths[1], PathBuf::from("/baz/qux"));
}

#[test]
fn test_pathbuf_cli_overrides_serde_with_use_value_parser() {
    let result = ConfigPathBufWithUseValueParser::conf_builder()
        .args(["test_app", "--paths", "/from/cli"])
        .doc(
            "test_doc",
            json!({
                "paths": ["/from/file"]
            }),
        )
        .try_parse()
        .unwrap();

    // CLI should override file
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], PathBuf::from("/from/cli"));
}

#[test]
#[cfg(unix)]
fn test_pathbuf_non_utf8_cli_with_use_value_parser() {
    use std::os::unix::ffi::OsStringExt;

    // Create non-UTF-8 path
    let non_utf8_path = OsString::from_vec(vec![b'/', b'f', b'o', b'o', 0xFF]);

    let result = ConfigPathBufWithUseValueParser::conf_builder()
        .args([
            OsString::from("test_app"),
            OsString::from("--paths"),
            non_utf8_path.clone(),
        ])
        .doc(
            "test_doc",
            json!({
                "paths": ["/from/file"]
            }),
        )
        .try_parse()
        .unwrap();

    // CLI should override file, and non-UTF-8 should work
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], PathBuf::from(non_utf8_path));
}

// Test Vec<OsString> with serde WITH use_value_parser
#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct ConfigOsStringWithUseValueParser {
    #[conf(repeat, long, serde(use_value_parser))]
    pub args: Vec<OsString>,
}

#[test]
fn test_osstring_serde_with_use_value_parser() {
    let result = ConfigOsStringWithUseValueParser::conf_builder()
        .args(["test_app"])
        .doc(
            "test_doc",
            json!({
                "args": ["hello", "world"]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.args.len(), 2);
    assert_eq!(result.args[0], "hello");
    assert_eq!(result.args[1], "world");
}

#[test]
#[cfg(unix)]
fn test_osstring_non_utf8_cli_with_use_value_parser() {
    use std::os::unix::ffi::OsStringExt;

    let non_utf8_arg = OsString::from_vec(vec![b'h', b'e', b'l', b'l', b'o', 0xFF]);

    let result = ConfigOsStringWithUseValueParser::conf_builder()
        .args([
            OsString::from("test_app"),
            OsString::from("--args"),
            non_utf8_arg.clone(),
        ])
        .doc(
            "test_doc",
            json!({
                "args": ["from_file"]
            }),
        )
        .try_parse()
        .unwrap();

    // CLI should override file
    assert_eq!(result.args.len(), 1);
    assert_eq!(result.args[0], non_utf8_arg);
}
