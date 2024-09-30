#![cfg(feature = "serde")]

mod common;
use common::{assert_multiline_eq, examples_dir, Example};
use std::str::from_utf8;

struct SerdeBasicExample {}
impl Example for SerdeBasicExample {
    const NAME: &'static str = "serde_basic";
}

#[test]
fn test_serde_basic_example_no_args() {
    let mut command = SerdeBasicExample::get_command();
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
fn test_serde_basic_example_some_invalid_args() {
    let mut command = SerdeBasicExample::get_command();
    let output = command
        .args(["--db-url=asdf:/"])
        .envs([("DB_RETRIES", "5")])
        .output()
        .unwrap();

    let expected = &"
error: Invalid value
  when parsing '--db-url' value 'asdf:/': invalid format

Help:
      --db-url <db.url>
          Database: Base URL
          [env: DB_URL]
"[1..];

    assert_eq!(output.status.code(), Some(2));
    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_help() {
    let mut command = SerdeBasicExample::get_command();
    let output = command.args(["--help"]).output().unwrap();

    let expected = &"
Configuration for model service

Usage: serde_basic [OPTIONS] [COMMAND]

Commands:
  run-migrations
  show-pending-migrations
  help                     Print this message or the help of the given subcommand(s)

Options:
      --listen-addr <listen_addr>    Listen address to bind to
                                     [env LISTEN_ADDR=]
                                     [default: 127.0.0.1:9090]
      --auth-url <auth.url>          Auth service: Base URL
                                     [env AUTH_URL=]
      --auth-retries <auth.retries>  Auth service: Number of retries
                                     [env AUTH_RETRIES=]
      --db-url <db.url>              Database: Base URL
                                     [env DB_URL=]
      --db-retries <db.retries>      Database: Number of retries
                                     [env DB_RETRIES=]
      --config <config_file>         Config file path
                                     [env CONFIG=]
  -h, --help                         Print help
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
fn test_serde_basic_example_success_args() {
    let mut command = SerdeBasicExample::get_command();
    let output = command
        .args([
            "--auth-url=https://example.com",
            "--auth-retries=7",
            "--db-url",
            "postgres://localhost/dev",
            "--db-retries",
            "9",
        ])
        .output()
        .unwrap();

    let expected = &r#"
ModelServiceConfig {
    listen_addr: 127.0.0.1:9090,
    auth: Some(
        HttpClientConfig {
            url: https://example.com/,
            retries: 7,
        },
    ),
    db: HttpClientConfig {
        url: postgres://localhost/dev,
        retries: 9,
    },
    config_file: None,
    command: None,
}
"#[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_success_env() {
    let mut command = SerdeBasicExample::get_command();
    let output = command
        .envs([
            ("LISTEN_ADDR", "0.0.0.0:7777"),
            ("DB_URL", "postgres://localhost/dev"),
            ("DB_RETRIES", "3"),
        ])
        .output()
        .unwrap();

    let expected = &r#"
ModelServiceConfig {
    listen_addr: 0.0.0.0:7777,
    auth: None,
    db: HttpClientConfig {
        url: postgres://localhost/dev,
        retries: 3,
    },
    config_file: None,
    command: None,
}
"#[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_success_args_and_file() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args([
            "--config",
            &toml_file,
            "--db-url",
            "postgres://localhost/dev",
        ])
        .output()
        .unwrap();

    let expected = &format!(
        r#"
ModelServiceConfig {{
    listen_addr: 0.0.0.0:80,
    auth: None,
    db: HttpClientConfig {{
        url: postgres://localhost/dev,
        retries: 3,
    }},
    config_file: Some(
        "{toml_file}",
    ),
    command: None,
}}
"#
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_success_args_and_file_shadowing() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args([
            "--config",
            &toml_file,
            "--db-url",
            "postgres://localhost/dev",
            "--db-retries=14",
        ])
        .output()
        .unwrap();

    let expected = &format!(
        r#"
ModelServiceConfig {{
    listen_addr: 0.0.0.0:80,
    auth: None,
    db: HttpClientConfig {{
        url: postgres://localhost/dev,
        retries: 14,
    }},
    config_file: Some(
        "{toml_file}",
    ),
    command: None,
}}
"#
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_success_and_args_env_and_file() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args(["--db-url", "postgres://localhost/dev"])
        .envs([
            ("CONFIG", toml_file.as_str()),
            ("AUTH_URL", "http://auth.svc.cluster"),
            ("AUTH_RETRIES", "3"),
        ])
        .output()
        .unwrap();

    let expected = &format!(
        r#"
ModelServiceConfig {{
    listen_addr: 0.0.0.0:80,
    auth: Some(
        HttpClientConfig {{
            url: http://auth.svc.cluster/,
            retries: 3,
        }},
    ),
    db: HttpClientConfig {{
        url: postgres://localhost/dev,
        retries: 3,
    }},
    config_file: Some(
        "{toml_file}",
    ),
    command: None,
}}
"#
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_success_and_args_env_and_file_shadowing() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args(["--db-url", "postgres://localhost/dev"])
        .envs([
            ("CONFIG", toml_file.as_str()),
            ("DB_URL", "postgres://db.svc.cluster"),
            ("DB_RETRIES", "82"),
            ("AUTH_URL", "http://auth.svc.cluster"),
            ("AUTH_RETRIES", "3"),
        ])
        .output()
        .unwrap();

    let expected = &format!(
        r#"
ModelServiceConfig {{
    listen_addr: 0.0.0.0:80,
    auth: Some(
        HttpClientConfig {{
            url: http://auth.svc.cluster/,
            retries: 3,
        }},
    ),
    db: HttpClientConfig {{
        url: postgres://localhost/dev,
        retries: 82,
    }},
    config_file: Some(
        "{toml_file}",
    ),
    command: None,
}}
"#
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_args_env_and_file_with_missing_and_invalid() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args(["--auth-url", "asdf:/"])
        .envs([
            ("CONFIG", toml_file.as_str()),
            ("DB_RETRIES", "82"),
            ("AUTH_RETRIES", "xxx"),
        ])
        .output()
        .unwrap();

    let expected = &format!(
        r#"
error: A required value was not provided
  env 'DB_URL', or '--db-url', must be provided
error: Invalid value
  when parsing '--auth-url' value 'asdf:/': invalid format
  when parsing env 'AUTH_RETRIES' value 'xxx': invalid digit found in string
"#
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(2),
        "stdout: {}",
        from_utf8(&output.stdout).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_args_env_and_file_with_missing_and_invalid2() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service2.toml";
    let output = command
        .args(["--auth-url", "asdf:/"])
        .envs([
            ("CONFIG", toml_file.as_str()),
            ("DB_RETRIES", "82"),
            ("AUTH_RETRIES", "xxx"),
        ])
        .output()
        .unwrap();

    let expected = &format!(
        r#"
error: A required value was not provided
  env 'DB_URL', or '--db-url', must be provided
error: Invalid value
  when parsing '--auth-url' value 'asdf:/': invalid format
  when parsing env 'AUTH_RETRIES' value 'xxx': invalid digit found in string
error: Parsing document
  Parsing {toml_file} (@ retries):
    invalid type: string "xxx", expected u32
    in `retries`

"#
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_subcommand_help() {
    let mut command = SerdeBasicExample::get_command();
    let output = command.args(["run-migrations", "--help"]).output().unwrap();

    let expected = &"
Usage: serde_basic run-migrations [OPTIONS]

Options:
      --sql-file <sql_file>  Path to migrations file (instead of embedded migrations)
                             [env SQL_FILE=]
  -h, --help                 Print help
"[1..];

    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_serde_basic_example_subcommand_invalid_args() {
    let mut command = SerdeBasicExample::get_command();
    let output = command
        .args(["run-migrations", "--db-url=postgres://localhost/dev"])
        .output()
        .unwrap();

    let expected = &"
error: unexpected argument '--db-url' found

Usage: serde_basic run-migrations [OPTIONS]

For more information, try '--help'.
"[1..];

    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_serde_basic_example_subcommand_missing_args() {
    let mut command = SerdeBasicExample::get_command();
    let output = command
        .args(["--db-url=postgres://localhost/dev", "run-migrations"])
        .output()
        .unwrap();

    let expected = &"
error: A required value was not provided
  env 'DB_RETRIES', or '--db-retries', must be provided

Help:
      --db-retries <db.retries>
          Database: Number of retries
          [env: DB_RETRIES]
"[1..];

    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_serde_basic_example_subcommand_args_and_env_success() {
    let mut command = SerdeBasicExample::get_command();
    let output = command
        .args(["--db-url=postgres://localhost/dev", "run-migrations"])
        .env("DB_RETRIES", "77")
        .output()
        .unwrap();

    let expected = &"
ModelServiceConfig {
    listen_addr: 127.0.0.1:9090,
    auth: None,
    db: HttpClientConfig {
        url: postgres://localhost/dev,
        retries: 77,
    },
    config_file: None,
    command: Some(
        RunMigrations(
            MigrationConfig {
                sql_file: None,
            },
        ),
    ),
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
fn test_serde_basic_example_subcommand_args_and_env_success2() {
    let mut command = SerdeBasicExample::get_command();
    let output = command
        .args([
            "--db-url=postgres://localhost/dev",
            "run-migrations",
            "--sql-file=foobar.sql",
        ])
        .env("DB_RETRIES", "77")
        .output()
        .unwrap();

    let expected = &"
ModelServiceConfig {
    listen_addr: 127.0.0.1:9090,
    auth: None,
    db: HttpClientConfig {
        url: postgres://localhost/dev,
        retries: 77,
    },
    config_file: None,
    command: Some(
        RunMigrations(
            MigrationConfig {
                sql_file: Some(
                    \"foobar.sql\",
                ),
            },
        ),
    ),
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
fn test_serde_basic_example_subcommand_args_and_file_success() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args([
            &format!("--config={toml_file}"),
            "--db-url=postgres://localhost/dev",
            "run-migrations",
        ])
        .output()
        .unwrap();

    let expected = &format!(
        "
ModelServiceConfig {{
    listen_addr: 0.0.0.0:80,
    auth: None,
    db: HttpClientConfig {{
        url: postgres://localhost/dev,
        retries: 3,
    }},
    config_file: Some(
        \"{toml_file}\",
    ),
    command: Some(
        RunMigrations(
            MigrationConfig {{
                sql_file: Some(
                    \"xxx.sql\",
                ),
            }},
        ),
    ),
}}
"
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_subcommand_args_and_file_success_shadowing() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args([
            "--db-url=postgres://localhost/dev",
            &format!("--config={toml_file}"),
            "run-migrations",
            "--sql-file",
            "foo.sql",
        ])
        .output()
        .unwrap();

    let expected = &format!(
        "
ModelServiceConfig {{
    listen_addr: 0.0.0.0:80,
    auth: None,
    db: HttpClientConfig {{
        url: postgres://localhost/dev,
        retries: 3,
    }},
    config_file: Some(
        \"{toml_file}\",
    ),
    command: Some(
        RunMigrations(
            MigrationConfig {{
                sql_file: Some(
                    \"foo.sql\",
                ),
            }},
        ),
    ),
}}
"
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_subcommand_args_and_env_file_success() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args(["--db-url=postgres://localhost/dev", "run-migrations"])
        .envs([
            ("CONFIG", toml_file.as_str()),
            ("AUTH_URL", "http://auth.service.cluster"),
            ("AUTH_RETRIES", "5"),
        ])
        .output()
        .unwrap();

    let expected = &format!(
        "
ModelServiceConfig {{
    listen_addr: 0.0.0.0:80,
    auth: Some(
        HttpClientConfig {{
            url: http://auth.service.cluster/,
            retries: 5,
        }},
    ),
    db: HttpClientConfig {{
        url: postgres://localhost/dev,
        retries: 3,
    }},
    config_file: Some(
        \"{toml_file}\",
    ),
    command: Some(
        RunMigrations(
            MigrationConfig {{
                sql_file: Some(
                    \"xxx.sql\",
                ),
            }},
        ),
    ),
}}
"
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_subcommand_args_and_env_file_success_shadowing() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args([
            "--db-url=postgres://localhost/dev",
            "run-migrations",
            "--sql-file=foo.sql",
        ])
        .envs([
            ("CONFIG", toml_file.as_str()),
            ("SQL_FILE", "bar.sql"),
            ("AUTH_URL", "http://auth.service.cluster"),
            ("AUTH_RETRIES", "5"),
        ])
        .output()
        .unwrap();

    let expected = &format!(
        "
ModelServiceConfig {{
    listen_addr: 0.0.0.0:80,
    auth: Some(
        HttpClientConfig {{
            url: http://auth.service.cluster/,
            retries: 5,
        }},
    ),
    db: HttpClientConfig {{
        url: postgres://localhost/dev,
        retries: 3,
    }},
    config_file: Some(
        \"{toml_file}\",
    ),
    command: Some(
        RunMigrations(
            MigrationConfig {{
                sql_file: Some(
                    \"foo.sql\",
                ),
            }},
        ),
    ),
}}
"
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        from_utf8(&output.stderr).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
}

#[test]
fn test_serde_basic_example_subcommand_args_and_env_file_missing_and_invalid() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service.toml";
    let output = command
        .args(["run-migrations", "--sql-file=foo.sql"])
        .envs([
            ("CONFIG", toml_file.as_str()),
            ("SQL_FILE", "bar.sql"),
            ("AUTH_RETRIES", "xxx"),
        ])
        .output()
        .unwrap();

    let expected = &"
error: A required value was not provided
  env 'AUTH_URL', or '--auth-url', must be provided
    because env 'AUTH_RETRIES' was provided (enabling argument group HttpClientConfig @ .auth)
  env 'DB_URL', or '--db-url', must be provided
error: Invalid value
  when parsing env 'AUTH_RETRIES' value 'xxx': invalid digit found in string
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
fn test_serde_basic_example_subcommand_args_and_env_file_missing_and_invalid2() {
    let mut command = SerdeBasicExample::get_command();
    let toml_file = examples_dir().to_string() + "/model_service2.toml";
    let output = command
        .args(["run-migrations", "--sql-file=foo.sql"])
        .envs([
            ("CONFIG", toml_file.as_str()),
            ("SQL_FILE", "bar.sql"),
            ("AUTH_RETRIES", "yyy"),
        ])
        .output()
        .unwrap();

    // This should NOT complain about unexpected arguments in model_service2.toml,
    // because the subcommand section is skipped unless the subcommand is active.
    let expected = &format!(
        "
error: A required value was not provided
  env 'AUTH_URL', or '--auth-url', must be provided
    because env 'AUTH_RETRIES' was provided (enabling argument group HttpClientConfig @ .auth)
  env 'DB_URL', or '--db-url', must be provided
error: Invalid value
  when parsing env 'AUTH_RETRIES' value 'yyy': invalid digit found in string
error: Parsing document
  Parsing {toml_file} (@ MigrationConfig):
    unknown field `unexpected_arg`, expected `sql_file`

  Parsing {toml_file} (@ retries):
    invalid type: string \"xxx\", expected u32
    in `retries`

"
    )[1..];

    assert_eq!(
        output.status.code(),
        Some(2),
        "stdout: {}",
        from_utf8(&output.stdout).unwrap()
    );
    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
}
