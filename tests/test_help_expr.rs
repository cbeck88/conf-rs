// `help = <expr>` on parameters, flags, and repeats, e.g. for translated help

use conf::{Conf, introspection::ProgramOptionMeta};
use std::cell::Cell;

mod common;
use common::*;

fn no_env() -> Vec<(&'static str, &'static str)> {
    vec![]
}

thread_local! {
    // Stand-in for the user's locale, which an i18n library would look up at runtime
    static LANG: Cell<&'static str> = const { Cell::new("en") };
}

fn tr(key: &str) -> String {
    let lang = LANG.with(Cell::get);
    match (lang, key) {
        ("en", "port") => "Port to listen on".into(),
        ("fr", "port") => "Port d'écoute".into(),
        ("en", "verbose") => "Print more output".into(),
        ("fr", "verbose") => "Afficher plus de détails".into(),
        ("en", "peers") => "Peers to connect to".into(),
        ("en", "token") => "Secret token".into(),
        ("en", "user") => "User name".into(),
        ("en", "multiline") => "First line\nSecond line".into(),
        _ => format!("<missing {key}>"),
    }
}

const STATIC_HELP: &str = "From a const";

fn help_of<C: Conf>(args: &[&str]) -> String {
    C::try_parse_from(args.iter().copied(), no_env())
        .err()
        .expect("--help should return an error")
        .to_string()
}

fn descriptions<C: Conf>() -> Vec<(String, Option<String>)> {
    C::program_options()
        .map(|opt| {
            (
                opt.id().to_string(),
                opt.description().map(|d| d.to_string()),
            )
        })
        .collect()
}

#[derive(Conf, Debug)]
#[allow(dead_code)]
struct Server {
    /// Doc comment for developers
    #[conf(long, default_value = "80", help = tr("port"))]
    port: u16,

    /// Doc comment for developers
    #[conf(long, help = tr("verbose"))]
    verbose: bool,

    #[conf(repeat, long, help = tr("peers"))]
    peers: Vec<String>,

    // Env-only, so its help goes in the "Environment variables" section instead
    #[conf(env, help = tr("token"))]
    token: Option<String>,

    #[conf(long, help = STATIC_HELP)]
    from_const: Option<String>,

    #[conf(long, help = format!("{} {}", "From", "format"))]
    from_format: Option<String>,

    /// Uses the doc comment
    #[conf(long)]
    no_help: Option<String>,
}

#[test]
fn help_expr_in_help_output() {
    LANG.with(|l| l.set("en"));
    let help = help_of::<Server>(&["prog", "--help"]);
    let expected = &"
Usage: prog [OPTIONS]

Options:
      --port <port>                Port to listen on
                                   [default: 80]
      --verbose                    Print more output
      --peers <peers>              Peers to connect to
      --from-const <from_const>    From a const
      --from-format <from_format>  From format
      --no-help <no_help>          Uses the doc comment
  -h, --help                       Print help

Environment variables:
      <token>
          Secret token
          [env: TOKEN=]
"[1..];
    assert_multiline_eq!(&help, expected);
}

#[test]
fn help_expr_is_evaluated_at_runtime() {
    LANG.with(|l| l.set("fr"));
    let help = help_of::<Server>(&["prog", "--help"]);
    assert!(help.contains("Port d'écoute"), "{help}");
    assert!(help.contains("Afficher plus de détails"), "{help}");
    assert!(!help.contains("Port to listen on"), "{help}");
    LANG.with(|l| l.set("en"));
}

#[test]
fn help_expr_in_program_options() {
    LANG.with(|l| l.set("en"));
    assert_eq!(
        descriptions::<Server>(),
        vec![
            ("port".into(), Some("Port to listen on".into())),
            ("verbose".into(), Some("Print more output".into())),
            ("peers".into(), Some("Peers to connect to".into())),
            ("token".into(), Some("Secret token".into())),
            ("from_const".into(), Some("From a const".into())),
            ("from_format".into(), Some("From format".into())),
            ("no_help".into(), Some("Uses the doc comment".into())),
        ]
    );
}

#[test]
fn help_expr_still_parses() {
    let s = Server::try_parse_from(
        [
            "prog",
            "--port",
            "8080",
            "--verbose",
            "--peers",
            "a",
            "--peers",
            "b",
        ],
        vec![("TOKEN", "xyz")],
    )
    .unwrap();
    assert_eq!(s.port, 8080);
    assert!(s.verbose);
    assert_eq!(s.peers, vec!["a", "b"]);
    assert_eq!(s.token.as_deref(), Some("xyz"));
}

#[derive(Conf, Debug)]
#[allow(dead_code)]
struct Credentials {
    #[conf(long, env, help = tr("user"))]
    user: Option<String>,

    #[conf(long, help = tr("multiline"))]
    note: Option<String>,
}

#[derive(Conf, Debug)]
#[allow(dead_code)]
struct Middle {
    #[conf(flatten, prefix, help_prefix = "(db)")]
    db: Credentials,
}

// Two levels of flattening, and an env_prefix, so that the transforms of several enclosing
// structs run on the same option after its help_prefix was applied
#[derive(Conf, Debug)]
#[conf(env_prefix = "APP_")]
#[allow(dead_code)]
struct Outer {
    #[conf(flatten, prefix, help_prefix = "(main)")]
    main: Middle,
}

#[test]
fn help_expr_with_flatten_help_prefix() {
    LANG.with(|l| l.set("en"));
    assert_eq!(
        descriptions::<Middle>(),
        vec![
            ("db.user".into(), Some("(db) User name".into())),
            (
                "db.note".into(),
                Some("(db)\nFirst line\nSecond line".into())
            ),
        ]
    );
    assert_eq!(
        descriptions::<Outer>(),
        vec![
            ("main.db.user".into(), Some("(main) (db) User name".into())),
            (
                "main.db.note".into(),
                Some("(main)\n(db)\nFirst line\nSecond line".into())
            ),
        ]
    );

    let help = help_of::<Outer>(&["prog", "--help"]);
    assert!(help.contains("(main) (db) User name"), "{help}");
    assert!(help.contains("[env APP_MAIN_DB_USER=]"), "{help}");
}
