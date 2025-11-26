/// Tests for env_delimiter ASCII validation with value_parser_os
///
/// The proc macro validates that env_delimiter must be ASCII when using value_parser_os
/// (either explicitly or implicitly via Vec<PathBuf> or Vec<OsString>).
/// This is required because OsStr splitting only works safely with ASCII delimiters.
use conf::Conf;
use std::ffi::OsString;
use std::path::PathBuf;

/// Test that ASCII delimiter works with value_parser_os
#[test]
fn test_ascii_delimiter_with_value_parser_os() {
    #[derive(Conf, Debug)]
    pub struct Config {
        #[conf(repeat, long, env, env_delimiter = ':', value_parser_os = |s: &std::ffi::OsStr| Ok::<_, String>(s.to_ascii_uppercase()))]
        pub items: Vec<OsString>,
    }

    let result = Config::try_parse_from::<&str, &str, &str>(
        vec!["test_app"],
        vec![("ITEMS", "hello:world:foo")],
    )
    .unwrap();
    assert_eq!(result.items.len(), 3);
    assert_eq!(result.items[0], "HELLO");
    assert_eq!(result.items[1], "WORLD");
    assert_eq!(result.items[2], "FOO");
}

/// Test that ASCII delimiter works with Vec<PathBuf> (auto-detected value_parser_os)
#[test]
fn test_ascii_delimiter_with_pathbuf() {
    #[derive(Conf, Debug)]
    pub struct Config {
        #[conf(repeat, long, env, env_delimiter = ':')]
        pub paths: Vec<PathBuf>,
    }

    let result = Config::try_parse_from::<&str, &str, &str>(
        vec!["test_app"],
        vec![("PATHS", "/usr/bin:/usr/local/bin:/home/user/bin")],
    )
    .unwrap();
    assert_eq!(result.paths.len(), 3);
    assert_eq!(result.paths[0], PathBuf::from("/usr/bin"));
    assert_eq!(result.paths[1], PathBuf::from("/usr/local/bin"));
    assert_eq!(result.paths[2], PathBuf::from("/home/user/bin"));
}

/// Test that ASCII delimiter works with Vec<OsString> (auto-detected value_parser_os)
#[test]
fn test_ascii_delimiter_with_osstring() {
    #[derive(Conf, Debug)]
    pub struct Config {
        #[conf(repeat, long, env, env_delimiter = '|')]
        pub items: Vec<OsString>,
    }

    let result =
        Config::try_parse_from::<&str, &str, &str>(vec!["test_app"], vec![("ITEMS", "a|b|c")])
            .unwrap();
    assert_eq!(result.items.len(), 3);
    assert_eq!(result.items[0], "a");
    assert_eq!(result.items[1], "b");
    assert_eq!(result.items[2], "c");
}

/// Test that non-OsStr types (like Vec<String>) can use non-ASCII delimiters
/// since they use the regular str-based value_parser which handles UTF-8
#[test]
fn test_non_ascii_delimiter_with_string_type() {
    #[derive(Conf, Debug)]
    pub struct Config {
        #[conf(repeat, long, env, env_delimiter = '日')]
        pub items: Vec<String>,
    }

    let result = Config::try_parse_from::<&str, &str, &str>(
        vec!["test_app"],
        vec![("ITEMS", "hello日world日foo")],
    )
    .unwrap();
    assert_eq!(result.items.len(), 3);
    assert_eq!(result.items[0], "hello");
    assert_eq!(result.items[1], "world");
    assert_eq!(result.items[2], "foo");
}
