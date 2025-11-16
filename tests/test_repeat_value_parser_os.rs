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
    // Test that env variables work with value_parser_os (no delimiter since it's incompatible)
    #[derive(Conf, Debug)]
    pub struct ConfigWithEnv {
        #[conf(repeat, long, env, value_parser_os = |s: &std::ffi::OsStr| Ok::<_, String>(s.to_ascii_uppercase()))]
        pub paths: Vec<std::ffi::OsString>,
    }

    let result = ConfigWithEnv::try_parse_from::<&str, &str, &str>(
        vec!["test_app"],
        vec![("PATHS", "test")],
    )
    .unwrap();
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], "TEST");
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
