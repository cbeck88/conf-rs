mod common;
use common::assert_multiline_eq;
use escargot::{CargoBuild, CargoRun};
use std::{process::Command, str::from_utf8, sync::OnceLock};

fn get_built_showcase_subcommands_example() -> Command {
    static ONCE: OnceLock<CargoRun> = OnceLock::new();
    ONCE.get_or_init(|| {
        CargoBuild::new()
            .example("showcase_subcommands")
            .run()
            .unwrap()
    })
    .command()
}

#[test]
fn test_showcase_example_no_args() {
    let mut command = get_built_showcase_subcommands_example();
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
fn test_showcase_example_some_invalid_args() {
    let mut command = get_built_showcase_subcommands_example();
    let output = command
        .args(["--db-url=asdf"])
        .envs([("DB_RETRIES", "5")])
        .output()
        .unwrap();

    let expected = &"
error: Invalid value
  when parsing '--db-url' value 'asdf': relative URL without a base

Help:
      --db-url <db.url>
          Database: Base URL
          [env: DB_URL]
"[1..];

    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_showcase_example_help() {
    let mut command = get_built_showcase_subcommands_example();
    let output = command.args(["--help"]).output().unwrap();

    let expected = &"
Configuration for model service

Usage: showcase_subcommands [OPTIONS] [COMMAND]

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
  -h, --help                         Print help
"[1..];

    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_showcase_example_success_args() {
    let mut command = get_built_showcase_subcommands_example();
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
            url: "https://example.com/",
            retries: 7,
        },
    ),
    db: HttpClientConfig {
        url: "postgres://localhost/dev",
        retries: 9,
    },
    command: None,
}
"#[1..];

    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_showcase_example_success_env() {
    let mut command = get_built_showcase_subcommands_example();
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
        url: "postgres://localhost/dev",
        retries: 3,
    },
    command: None,
}
"#[1..];

    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_showcase_example_subcommand_help() {
    let mut command = get_built_showcase_subcommands_example();
    let output = command.args(["run-migrations", "--help"]).output().unwrap();

    let expected = &"
Usage: showcase_subcommands run-migrations [OPTIONS]

Options:
      --migrations <migrations>  Path to migrations file (instead of embedded migrations)
                                 [env MIGRATIONS=]
  -h, --help                     Print help
"[1..];

    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_showcase_example_subcommand_invalid_args() {
    let mut command = get_built_showcase_subcommands_example();
    let output = command
        .args(["run-migrations", "--db-url=postgres://localhost/dev"])
        .output()
        .unwrap();

    let expected = &"
error: unexpected argument '--db-url' found

Usage: showcase_subcommands run-migrations [OPTIONS]

For more information, try '--help'.
"[1..];

    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_showcase_example_subcommand_missing_args() {
    let mut command = get_built_showcase_subcommands_example();
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
fn test_showcase_example_subcommand_success() {
    let mut command = get_built_showcase_subcommands_example();
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
        url: \"postgres://localhost/dev\",
        retries: 77,
    },
    command: Some(
        RunMigrations(
            MigrationConfig {
                migrations: None,
            },
        ),
    ),
}
"[1..];

    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_showcase_example_subcommand_success2() {
    let mut command = get_built_showcase_subcommands_example();
    let output = command
        .args([
            "--db-url=postgres://localhost/dev",
            "run-migrations",
            "--migrations=foobar.sql",
        ])
        .env("DB_RETRIES", "77")
        .output()
        .unwrap();

    let expected = &"
ModelServiceConfig {
    listen_addr: 127.0.0.1:9090,
    auth: None,
    db: HttpClientConfig {
        url: \"postgres://localhost/dev\",
        retries: 77,
    },
    command: Some(
        RunMigrations(
            MigrationConfig {
                migrations: Some(
                    \"foobar.sql\",
                ),
            },
        ),
    ),
}
"[1..];

    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(0));
}
