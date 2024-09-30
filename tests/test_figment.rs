#![cfg(feature = "serde")]

mod common;
use common::{assert_multiline_eq, examples_dir, Example};
use std::str::from_utf8;

struct FigmentExample {}
impl Example for FigmentExample {
    const NAME: &'static str = "figment";
}

#[test]
fn test_figment_example_no_args() {
    let mut command = FigmentExample::get_command();
    let output = command.output().unwrap();

    let expected = &"
error: A required value was not provided
  env 'DB_RETRIES', or '--db-retries', must be provided
  env 'DB_URL', or '--db-url', must be provided
"[1..];

    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_figment_example_one_file_expected_failure() {
    let mut command = FigmentExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .envs([("TOML", toml_file.as_str())])
        .output()
        .unwrap();

    let expected = &"
error: A required value was not provided
  env 'DB_URL', or '--db-url', must be provided

Help:
      --db-url <db.url>
          Database: Base URL
          [env: DB_URL]
"[1..];

    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_figment_example_one_file_success() {
    let mut command = FigmentExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args(["--db-url", "postgres://localhost/dev"])
        .envs([("TOML", toml_file.as_str())])
        .output()
        .unwrap();

    let expected = &"
ModelServiceConfig {
    listen_addr: 0.0.0.0:80,
    auth: None,
    db: HttpClientConfig {
        url: postgres://localhost/dev,
        retries: 3,
    },
    config_file: None,
    command: None,
}
"[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_figment_example_two_file_success() {
    let mut command = FigmentExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let toml_file2 = examples_dir().to_string() + "/model_service2.toml";
    let output = command
        .args(["--db-url", "postgres://localhost/dev"])
        .envs([("TOML", toml_file2.as_str()), ("TOML2", toml_file.as_str())])
        .output()
        .unwrap();

    let expected = &"
ModelServiceConfig {
    listen_addr: 0.0.0.0:80,
    auth: None,
    db: HttpClientConfig {
        url: postgres://localhost/dev,
        retries: 3,
    },
    config_file: None,
    command: None,
}
"[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_figment_example_two_file_expected_failure_other_order() {
    let mut command = FigmentExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let toml_file2 = examples_dir().to_string() + "/model_service2.toml";
    let output = command
        .args(["--db-url", "postgres://localhost/dev"])
        .envs([("TOML", toml_file.as_str()), ("TOML2", toml_file2.as_str())])
        .output()
        .unwrap();

    let expected = &"
error: Parsing document
  Parsing files (@ retries): invalid type: found string \"xxx\", expected u32
"[1..];

    assert_eq!(
        output.status.code(),
        Some(2),
        "stdout: {}",
        from_utf8(&output.stdout).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
}

#[test]
fn test_figment_example_two_file_shadowing_failure_from_env() {
    let mut command = FigmentExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let toml_file2 = examples_dir().to_string() + "/model_service2.toml";
    let output = command
        .args(["--db-url", "postgres://localhost/dev"])
        .envs([
            ("TOML", toml_file.as_str()),
            ("TOML2", toml_file2.as_str()),
            ("DB_RETRIES", "7"),
        ])
        .output()
        .unwrap();

    // Trying to shadow DB_RETRIES doesn't prevent the error because the serde parser still fails,
    // at least, that's the current behavior.
    //
    // Whether it's desirable, TBD.
    // Note that we don't run the value_parser, but we do call serde next_value.
    //
    // The pros are, if your config file has junk in it, you find out now and not later
    // when you remove an environment variable.
    // The con is, a shadowed value is preventing the application from starting up.
    let expected = &"
error: Parsing document
  Parsing files (@ retries): invalid type: found string \"xxx\", expected u32
"[1..];

    assert_eq!(
        output.status.code(),
        Some(2),
        "stdout: {}",
        from_utf8(&output.stdout).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
}

#[test]
fn test_figment_example_three_file_shadowing_success() {
    let mut command = FigmentExample::get_command();
    let json_file = examples_dir().to_string() + "/model_service.json";
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let toml_file2 = examples_dir().to_string() + "/model_service2.toml";
    let output = command
        .args(["--db-url", "postgres://localhost/dev"])
        .envs([
            ("TOML", toml_file.as_str()),
            ("TOML2", toml_file2.as_str()),
            ("JSON", json_file.as_str()),
            ("DB_RETRIES", "7"),
        ])
        .output()
        .unwrap();

    // This succeeds because the JSON overwrites the bad value in TOML2.
    // This prevents us from trying to deserialize the bad value.
    let expected = &"
ModelServiceConfig {
    listen_addr: 0.0.0.0:81,
    auth: None,
    db: HttpClientConfig {
        url: postgres://localhost/dev,
        retries: 7,
    },
    config_file: None,
    command: None,
}
"[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}
