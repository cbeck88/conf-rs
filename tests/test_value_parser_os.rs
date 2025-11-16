mod common;
use common::*;

use conf::Conf;
use std::ffi::OsString;
use std::path::PathBuf;

// Test automatic PathBuf handling
#[derive(Conf, Debug)]
struct TestPathBuf {
    #[conf(long, env)]
    path: PathBuf,

    #[conf(long, env)]
    optional_path: Option<PathBuf>,
}

#[test]
fn test_pathbuf_auto() {
    // Test with command line argument
    let result = TestPathBuf::try_parse_from::<&str, &str, &str>(
        vec![".", "--path=/foo/bar", "--optional-path=/baz/qux"],
        vec![],
    )
    .unwrap();

    assert_eq!(result.path, PathBuf::from("/foo/bar"));
    assert_eq!(result.optional_path, Some(PathBuf::from("/baz/qux")));

    // Test with environment variable
    let result = TestPathBuf::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("PATH", "/env/path"), ("OPTIONAL_PATH", "/env/opt")],
    )
    .unwrap();

    assert_eq!(result.path, PathBuf::from("/env/path"));
    assert_eq!(result.optional_path, Some(PathBuf::from("/env/opt")));

    // Test optional path not provided
    let result =
        TestPathBuf::try_parse_from::<&str, &str, &str>(vec![".", "--path=/foo/bar"], vec![])
            .unwrap();

    assert_eq!(result.path, PathBuf::from("/foo/bar"));
    assert_eq!(result.optional_path, None);

    // Test required path not provided
    assert_error_contains_text!(
        TestPathBuf::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["required value was not provided", "env 'PATH'"]
    );
}

// Test that PathBuf can handle non-UTF8 data
#[test]
fn test_pathbuf_non_utf8() {
    use std::os::unix::ffi::OsStringExt;

    // Create a path with bytes that are valid UTF-16LE but invalid UTF-8
    // 0x3D 0xD8 0x4A 0xDC represents UTF-16LE surrogate pair for U+1034A (𐍊)
    let mut invalid_path = OsString::from("/tmp/file_");
    invalid_path.push(OsString::from_vec(vec![0x3D, 0xD8, 0x4A, 0xDC]));

    let mut args = vec![OsString::from(".")];
    let mut path_arg = OsString::from("--path=");
    path_arg.push(&invalid_path);
    args.push(path_arg);

    let result = TestPathBuf::try_parse_from::<OsString, &str, &str>(args, vec![]);

    // This should succeed because PathBuf uses OsStr parser
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.path.as_os_str(), &invalid_path);
}

// Test automatic OsString handling
#[derive(Conf, Debug)]
struct TestOsString {
    #[conf(long, env)]
    os_str: OsString,

    #[conf(long, env)]
    optional_os: Option<OsString>,
}

#[test]
fn test_osstring_auto() {
    // Test with command line argument
    let result = TestOsString::try_parse_from::<&str, &str, &str>(
        vec![".", "--os-str=hello", "--optional-os=world"],
        vec![],
    )
    .unwrap();

    assert_eq!(result.os_str, OsString::from("hello"));
    assert_eq!(result.optional_os, Some(OsString::from("world")));

    // Test with environment variable
    let result = TestOsString::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("OS_STR", "env_hello"), ("OPTIONAL_OS", "env_world")],
    )
    .unwrap();

    assert_eq!(result.os_str, OsString::from("env_hello"));
    assert_eq!(result.optional_os, Some(OsString::from("env_world")));

    // Test optional not provided
    let result =
        TestOsString::try_parse_from::<&str, &str, &str>(vec![".", "--os-str=hello"], vec![])
            .unwrap();

    assert_eq!(result.os_str, OsString::from("hello"));
    assert_eq!(result.optional_os, None);
}

// Test that OsString can handle non-UTF8 data
#[test]
fn test_osstring_non_utf8() {
    use std::os::unix::ffi::OsStringExt;

    // Create bytes that are valid UTF-16LE but invalid UTF-8
    // 0xFF 0xFE is UTF-16LE BOM, followed by 0x00 0xD8 (high surrogate start)
    let invalid_utf8 = OsString::from_vec(vec![0xFF, 0xFE, 0x00, 0xD8]);

    let mut args = vec![OsString::from(".")];
    let mut os_arg = OsString::from("--os-str=");
    os_arg.push(&invalid_utf8);
    args.push(os_arg);

    let result = TestOsString::try_parse_from::<OsString, &str, &str>(args, vec![]);

    // This should succeed because OsString uses OsStr parser
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.os_str, invalid_utf8);
}

// Test explicit value_parser_os with custom parser
fn uppercase_osstr(s: &std::ffi::OsStr) -> Result<String, &'static str> {
    s.to_str().map(|s| s.to_uppercase()).ok_or("Invalid UTF-8")
}

#[derive(Conf, Debug)]
struct TestValueParserOs {
    #[conf(long, env, value_parser_os = uppercase_osstr)]
    text: String,
}

#[test]
fn test_value_parser_os_custom() {
    let result =
        TestValueParserOs::try_parse_from::<&str, &str, &str>(vec![".", "--text=hello"], vec![])
            .unwrap();

    assert_eq!(result.text, "HELLO");

    let result =
        TestValueParserOs::try_parse_from::<&str, &str, &str>(vec!["."], vec![("TEXT", "world")])
            .unwrap();

    assert_eq!(result.text, "WORLD");
}

// Test non-UTF-8 environment variables with PathBuf
#[test]
fn test_pathbuf_non_utf8_env() {
    use std::os::unix::ffi::OsStringExt;

    // Create a path with bytes that are valid UTF-16LE but invalid UTF-8
    // 0x3D 0xD8 0x4A 0xDC represents UTF-16LE surrogate pair for U+1034A (𐍊)
    let mut invalid_path = OsString::from("/tmp/file_");
    invalid_path.push(OsString::from_vec(vec![0x3D, 0xD8, 0x4A, 0xDC]));

    let result = TestPathBuf::try_parse_from::<&str, OsString, OsString>(
        vec!["."],
        vec![("PATH".into(), invalid_path.clone())],
    );

    // This should succeed because PathBuf uses OsStr parser
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.path.as_os_str(), &invalid_path);
}

// Test non-UTF-8 environment variables with OsString
#[test]
fn test_osstring_non_utf8_env() {
    use std::os::unix::ffi::OsStringExt;

    // Create bytes that are valid UTF-16LE but invalid UTF-8
    // 0xFF 0xFE is UTF-16LE BOM, followed by 0x00 0xD8 (high surrogate start)
    let invalid_utf8 = OsString::from_vec(vec![0xFF, 0xFE, 0x00, 0xD8]);

    let result = TestOsString::try_parse_from::<&str, OsString, OsString>(
        vec!["."],
        vec![("OS_STR".into(), invalid_utf8.clone())],
    );

    // This should succeed because OsString uses OsStr parser
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.os_str, invalid_utf8);
}

// Test that explicit value_parser overrides auto-detection for PathBuf
#[derive(Conf, Debug)]
struct TestPathBufExplicitValueParser {
    #[conf(long, value_parser = |s: &str| -> Result<PathBuf, String> {
        // Custom parser that uppercases the path
        Ok(PathBuf::from(s.to_uppercase()))
    })]
    path: PathBuf,
}

#[test]
fn test_pathbuf_explicit_value_parser_overrides_auto_detection() {
    let result = TestPathBufExplicitValueParser::try_parse_from::<&str, &str, &str>(
        vec![".", "--path=/foo/bar"],
        vec![],
    )
    .unwrap();

    // The explicit value_parser uppercases, so auto-detection is not used
    assert_eq!(result.path, PathBuf::from("/FOO/BAR"));
}

// Test that explicit value_parser_os overrides auto-detection for PathBuf
#[derive(Conf, Debug)]
struct TestPathBufExplicitValueParserOs {
    #[conf(long, value_parser_os = |s: &std::ffi::OsStr| -> Result<PathBuf, String> {
        // Custom parser that adds a prefix
        let mut path = PathBuf::from("/custom");
        path.push(s);
        Ok(path)
    })]
    path: PathBuf,
}

#[test]
fn test_pathbuf_explicit_value_parser_os_overrides_auto_detection() {
    let result = TestPathBufExplicitValueParserOs::try_parse_from::<&str, &str, &str>(
        vec![".", "--path=foo"],
        vec![],
    )
    .unwrap();

    // The explicit value_parser_os adds /custom/, so auto-detection is not used
    assert_eq!(result.path, PathBuf::from("/custom/foo"));
}

// Test that explicit value_parser overrides auto-detection for OsString
#[derive(Conf, Debug)]
struct TestOsStringExplicitValueParser {
    #[conf(long, value_parser = |s: &str| -> Result<OsString, String> {
        // Custom parser that reverses the string
        Ok(OsString::from(s.chars().rev().collect::<String>()))
    })]
    os_str: OsString,
}

#[test]
fn test_osstring_explicit_value_parser_overrides_auto_detection() {
    let result = TestOsStringExplicitValueParser::try_parse_from::<&str, &str, &str>(
        vec![".", "--os-str=hello"],
        vec![],
    )
    .unwrap();

    // The explicit value_parser reverses, so auto-detection is not used
    assert_eq!(result.os_str, OsString::from("olleh"));
}

// Test UTF-8 error precedence: we should only validate the value we actually use
#[derive(Conf, Debug)]
struct TestUtf8Precedence {
    #[conf(long, env)]
    value: String,
}

#[test]
fn test_utf8_error_precedence() {
    use std::os::unix::ffi::OsStringExt;

    // Helper to create bytes that are valid UTF-16LE but invalid UTF-8
    // 0xFF 0xFE is the UTF-16LE BOM
    let invalid_utf8 = OsString::from_vec(vec![0xFF, 0xFE]);

    // Case 1: Valid UTF-8 in args, valid UTF-8 in env
    // Should use args successfully
    let result = TestUtf8Precedence::try_parse_from::<OsString, &str, &str>(
        vec![OsString::from("."), OsString::from("--value=from_args")],
        vec![("VALUE", "from_env")],
    )
    .unwrap();
    assert_eq!(result.value, "from_args");

    // Case 2: Valid UTF-8 in args, invalid UTF-8 in env
    // Should use args successfully (never validates env)
    let result = TestUtf8Precedence::try_parse_from::<OsString, &str, &str>(
        vec![OsString::from("."), OsString::from("--value=from_args")],
        vec![],
    )
    .unwrap();
    assert_eq!(result.value, "from_args");

    // Case 3: Invalid UTF-8 in args, valid UTF-8 in env
    // Should fail with error about args (never tries env)
    let mut args = vec![OsString::from(".")];
    let mut invalid_arg = OsString::from("--value=");
    invalid_arg.push(&invalid_utf8);
    args.push(invalid_arg);

    let result = TestUtf8Precedence::try_parse_from::<OsString, &str, &str>(
        args,
        vec![("VALUE", "from_env")],
    );
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid UTF-8") || err_msg.contains("invalid"));

    // Case 4: Invalid UTF-8 in args, invalid UTF-8 in env
    // Should fail with error about args (never gets to env)
    let mut args = vec![OsString::from(".")];
    let mut invalid_arg = OsString::from("--value=");
    invalid_arg.push(&invalid_utf8);
    args.push(invalid_arg);

    let result = TestUtf8Precedence::try_parse_from::<OsString, &str, &str>(args, vec![]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid UTF-8") || err_msg.contains("invalid"));

    // Case 5: No args, valid UTF-8 in env
    // Should use env successfully
    let result = TestUtf8Precedence::try_parse_from::<OsString, &str, &str>(
        vec![OsString::from(".")],
        vec![("VALUE", "from_env")],
    )
    .unwrap();
    assert_eq!(result.value, "from_env");
}
