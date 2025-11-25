use conf::Conf;

#[derive(Conf, Debug)]
pub struct Config {
    #[conf(repeat, long, value_parser_os = |s: &std::ffi::OsStr| Ok::<_, String>(s.to_ascii_uppercase()))]
    pub paths: Vec<std::ffi::OsString>,
}

#[test]
fn test_repeat_value_parser_os_lambda() {
    let result = Config::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "--paths", "hello", "--paths", "world"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.paths.len(), 2);
    assert_eq!(result.paths[0], "HELLO");
    assert_eq!(result.paths[1], "WORLD");
}

#[test]
fn test_repeat_value_parser_os_env() {
    // Test that env variables work with value_parser_os with default comma delimiter
    #[derive(Conf, Debug)]
    pub struct ConfigWithEnv {
        #[conf(repeat, long, env, value_parser_os = |s: &std::ffi::OsStr| Ok::<_, String>(s.to_ascii_uppercase()))]
        pub paths: Vec<std::ffi::OsString>,
    }

    // Single value (no comma)
    let result = ConfigWithEnv::try_parse_from::<&str, &str, &str>(
        vec!["test_app"],
        vec![("PATHS", "test")],
    )
    .unwrap();
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], "TEST");

    // Multiple values separated by comma (default delimiter)
    let result = ConfigWithEnv::try_parse_from::<&str, &str, &str>(
        vec!["test_app"],
        vec![("PATHS", "hello,world,foo")],
    )
    .unwrap();
    assert_eq!(result.paths.len(), 3);
    assert_eq!(result.paths[0], "HELLO");
    assert_eq!(result.paths[1], "WORLD");
    assert_eq!(result.paths[2], "FOO");
}

#[test]
fn test_repeat_value_parser_os_env_custom_delimiter() {
    // Test custom env_delimiter with value_parser_os
    #[derive(Conf, Debug)]
    pub struct ConfigWithCustomDelim {
        #[conf(repeat, long, env, env_delimiter = ':', value_parser_os = |s: &std::ffi::OsStr| Ok::<_, String>(s.to_ascii_uppercase()))]
        pub paths: Vec<std::ffi::OsString>,
    }

    let result = ConfigWithCustomDelim::try_parse_from::<&str, &str, &str>(
        vec!["test_app"],
        vec![("PATHS", "hello:world:foo")],
    )
    .unwrap();
    assert_eq!(result.paths.len(), 3);
    assert_eq!(result.paths[0], "HELLO");
    assert_eq!(result.paths[1], "WORLD");
    assert_eq!(result.paths[2], "FOO");

    // Comma should NOT split since we specified ':' as delimiter
    let result = ConfigWithCustomDelim::try_parse_from::<&str, &str, &str>(
        vec!["test_app"],
        vec![("PATHS", "hello,world")],
    )
    .unwrap();
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], "HELLO,WORLD");
}

#[test]
fn test_repeat_value_parser_os_env_no_delimiter() {
    // Test no_env_delimiter with value_parser_os
    #[derive(Conf, Debug)]
    pub struct ConfigNoDelim {
        #[conf(repeat, long, env, no_env_delimiter, value_parser_os = |s: &std::ffi::OsStr| Ok::<_, String>(s.to_ascii_uppercase()))]
        pub paths: Vec<std::ffi::OsString>,
    }

    // With no_env_delimiter, the entire string is one value
    let result = ConfigNoDelim::try_parse_from::<&str, &str, &str>(
        vec!["test_app"],
        vec![("PATHS", "hello,world,foo")],
    )
    .unwrap();
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], "HELLO,WORLD,FOO");
}

#[test]
fn test_repeat_value_parser_os_with_args_override_env() {
    // Test that args shadow env variables
    #[derive(Conf, Debug)]
    pub struct ConfigWithEnv2 {
        #[conf(repeat, long, env, value_parser_os = |s: &std::ffi::OsStr| Ok::<_, String>(s.to_ascii_uppercase()))]
        pub paths: Vec<std::ffi::OsString>,
    }

    let result = ConfigWithEnv2::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "--paths", "arg_value"],
        vec![("PATHS", "env_value")],
    )
    .unwrap();
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], "ARG_VALUE");
}
