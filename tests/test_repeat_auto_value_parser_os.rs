use conf::Conf;
use std::ffi::OsString;
use std::path::PathBuf;

// Test that Vec<PathBuf> auto-detects and uses value_parser_os
#[derive(Conf, Debug)]
pub struct ConfigPathBuf {
    #[conf(repeat, long)]
    pub paths: Vec<PathBuf>,
}

#[test]
fn test_repeat_pathbuf_auto_detection() {
    let result = ConfigPathBuf::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "--paths", "/foo/bar", "--paths", "/baz/qux"],
        vec![],
    )
    .unwrap();

    assert_eq!(result.paths.len(), 2);
    assert_eq!(result.paths[0], PathBuf::from("/foo/bar"));
    assert_eq!(result.paths[1], PathBuf::from("/baz/qux"));
}

#[test]
#[cfg(unix)]
fn test_repeat_pathbuf_non_utf8() {
    use std::os::unix::ffi::OsStringExt;

    // Create non-UTF-8 path
    let non_utf8_path = OsString::from_vec(vec![b'/', b'f', b'o', b'o', 0xFF, 0xFE]);

    let result = ConfigPathBuf::conf_builder()
        .args([
            OsString::from("test_app"),
            OsString::from("--paths"),
            non_utf8_path.clone(),
        ])
        .try_parse()
        .unwrap();

    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], PathBuf::from(non_utf8_path));
}

// Test that Vec<OsString> auto-detects and uses value_parser_os
#[derive(Conf, Debug)]
pub struct ConfigOsString {
    #[conf(repeat, long)]
    pub args: Vec<OsString>,
}

#[test]
fn test_repeat_osstring_auto_detection() {
    let result = ConfigOsString::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "--args", "hello", "--args", "world"],
        vec![],
    )
    .unwrap();

    assert_eq!(result.args.len(), 2);
    assert_eq!(result.args[0], "hello");
    assert_eq!(result.args[1], "world");
}

#[test]
#[cfg(unix)]
fn test_repeat_osstring_non_utf8() {
    use std::os::unix::ffi::OsStringExt;

    // Create non-UTF-8 OsString
    let non_utf8_arg1 = OsString::from_vec(vec![b'h', b'e', b'l', b'l', b'o', 0xFF]);
    let non_utf8_arg2 = OsString::from_vec(vec![b'w', b'o', b'r', b'l', b'd', 0xFE, 0xFD]);

    let result = ConfigOsString::conf_builder()
        .args([
            OsString::from("test_app"),
            OsString::from("--args"),
            non_utf8_arg1.clone(),
            OsString::from("--args"),
            non_utf8_arg2.clone(),
        ])
        .try_parse()
        .unwrap();

    assert_eq!(result.args.len(), 2);
    assert_eq!(result.args[0], non_utf8_arg1);
    assert_eq!(result.args[1], non_utf8_arg2);
}

// Test that positional Vec<PathBuf> works
#[derive(Conf, Debug)]
pub struct ConfigPositionalPaths {
    #[conf(repeat, pos)]
    pub files: Vec<PathBuf>,
}

#[test]
fn test_repeat_pathbuf_positional() {
    let result = ConfigPositionalPaths::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "file1.txt", "file2.txt", "file3.txt"],
        vec![],
    )
    .unwrap();

    assert_eq!(result.files.len(), 3);
    assert_eq!(result.files[0], PathBuf::from("file1.txt"));
    assert_eq!(result.files[1], PathBuf::from("file2.txt"));
    assert_eq!(result.files[2], PathBuf::from("file3.txt"));
}

// Test that you can bypass auto-detection with fully qualified path
#[derive(Conf, Debug)]
pub struct ConfigFullyQualifiedPath {
    #[conf(repeat, long)]
    pub paths: Vec<std::path::PathBuf>,
}

// This should still work with auto-detection disabled, but using FromStr
// which will fail because PathBuf doesn't implement FromStr
// So we're just checking that it compiles
#[test]
fn test_fully_qualified_path_compiles() {
    // This uses the fully qualified path std::path::PathBuf which should bypass
    // auto-detection. However, PathBuf still needs a value_parser since it doesn't
    // implement FromStr. This test just verifies compilation.
    let _ = ConfigFullyQualifiedPath::try_parse_from::<&str, &str, &str>(vec!["test_app"], vec![]);
}

// Test that explicit value_parser overrides auto-detection
#[derive(Conf, Debug)]
pub struct ConfigExplicitValueParser {
    #[conf(repeat, long, value_parser = |s: &str| -> Result<PathBuf, String> {
        // Custom parser that uppercases the path
        Ok(PathBuf::from(s.to_uppercase()))
    })]
    pub paths: Vec<PathBuf>,
}

#[test]
fn test_explicit_value_parser_overrides_auto_detection() {
    let result = ConfigExplicitValueParser::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "--paths", "/foo/bar", "--paths", "/baz"],
        vec![],
    )
    .unwrap();

    assert_eq!(result.paths.len(), 2);
    // The explicit value_parser uppercases, so auto-detection is not used
    assert_eq!(result.paths[0], PathBuf::from("/FOO/BAR"));
    assert_eq!(result.paths[1], PathBuf::from("/BAZ"));
}

// Test that explicit value_parser_os overrides auto-detection
#[derive(Conf, Debug)]
pub struct ConfigExplicitValueParserOs {
    #[conf(repeat, long, value_parser_os = |s: &std::ffi::OsStr| -> Result<PathBuf, String> {
        // Custom parser that adds a prefix
        let mut path = PathBuf::from("/prefix");
        path.push(s);
        Ok(path)
    })]
    pub paths: Vec<PathBuf>,
}

#[test]
fn test_explicit_value_parser_os_overrides_auto_detection() {
    let result = ConfigExplicitValueParserOs::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "--paths", "foo", "--paths", "bar"],
        vec![],
    )
    .unwrap();

    assert_eq!(result.paths.len(), 2);
    // The explicit value_parser_os adds /prefix/, so auto-detection is not used
    assert_eq!(result.paths[0], PathBuf::from("/prefix/foo"));
    assert_eq!(result.paths[1], PathBuf::from("/prefix/bar"));
}
