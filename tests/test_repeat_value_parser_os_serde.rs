#![cfg(feature = "serde")]

use conf::Conf;
use serde_json::json;
use std::ffi::OsString;

#[derive(Conf, Debug, serde::Deserialize)]
#[conf(serde)]
pub struct Config {
    #[conf(repeat, long, env, value_parser_os = |s: &std::ffi::OsStr| Ok::<_, String>(s.to_ascii_uppercase()), serde(use_value_parser))]
    pub paths: Vec<OsString>,
}

#[test]
fn test_repeat_value_parser_os_with_serde() {
    let result = Config::conf_builder()
        .args(["test_app"])
        .doc(
            "test_doc",
            json!({
                "paths": ["foo", "bar", "baz"]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.paths.len(), 3);
    assert_eq!(result.paths[0], "FOO");
    assert_eq!(result.paths[1], "BAR");
    assert_eq!(result.paths[2], "BAZ");
}

#[test]
fn test_repeat_value_parser_os_cli_overrides_serde() {
    let result = Config::conf_builder()
        .args(["test_app", "--paths", "from_cli"])
        .doc(
            "test_doc",
            json!({
                "paths": ["from_file"]
            }),
        )
        .try_parse()
        .unwrap();

    // CLI should override file
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], "FROM_CLI");
}

#[test]
fn test_repeat_value_parser_os_env_overrides_serde() {
    let result = Config::conf_builder()
        .args(["test_app"])
        .env([("PATHS", "from_env")])
        .doc(
            "test_doc",
            json!({
                "paths": ["from_file"]
            }),
        )
        .try_parse()
        .unwrap();

    // Env should override file
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0], "FROM_ENV");
}
