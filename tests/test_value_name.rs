#![allow(dead_code)]

use conf::{Conf, Parser};

mod common;
use common::assert_multiline_eq;

#[derive(Conf)]
struct DbOptions {
    /// Base URL
    #[conf(long, env, value_name = "URL")]
    url: String,

    /// Number of retries
    #[conf(long, default_value = "3")]
    retries: u32,
}

#[derive(Conf)]
struct ValueNameOptions {
    /// Output file
    #[conf(short, long, value_name = "FILE")]
    output: Option<String>,

    /// Peers to connect to
    #[conf(repeat, long, value_name = "PEER")]
    peer: Vec<String>,

    #[conf(flatten, prefix, help_prefix = "Database:")]
    db: DbOptions,

    /// Input file
    #[conf(pos, value_name = "INPUT")]
    input: String,

    /// Extra args
    #[conf(repeat, pos, value_name = "ARG")]
    extra: Vec<String>,
}

#[test]
fn test_value_name_help() {
    let parser_config = ValueNameOptions::get_parser_config().unwrap();
    let opts = ValueNameOptions::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();

    let env = Default::default();
    let parser = Parser::new(parser_config, opts, &[], &env).unwrap();

    let clap_help = parser.render_clap_help();
    let expected = &"
Usage: . [OPTIONS] [INPUT] [ARG]...

Arguments:
  [INPUT]   Input file
  [ARG]...  Extra args

Options:
  -o, --output <FILE>            Output file
      --peer <PEER>              Peers to connect to
      --db-url <URL>             Database: Base URL
                                 [env DB_URL=]
      --db-retries <db.retries>  Database: Number of retries
                                 [default: 3]
  -h, --help                     Print help
"[1..];
    assert_multiline_eq!(&clap_help, expected);
}

#[test]
fn test_value_name_program_options() {
    let opts = ValueNameOptions::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();
    let value_name = |id: &str| {
        opts.iter()
            .find(|opt| opt.id == id)
            .unwrap_or_else(|| panic!("no option with id {id}"))
            .value_name
            .as_deref()
    };

    assert_eq!(value_name("output"), Some("FILE"));
    assert_eq!(value_name("peer"), Some("PEER"));
    // value_name is not prefixed when flattening, but the id is
    assert_eq!(value_name("db.url"), Some("URL"));
    assert_eq!(value_name("db.retries"), None);
    assert_eq!(value_name("input"), Some("INPUT"));
    assert_eq!(value_name("extra"), Some("ARG"));
}

#[test]
fn test_value_name_does_not_affect_parsing() {
    let opts = ValueNameOptions::try_parse_from(
        [
            ".", "-o", "out.txt", "--peer", "a", "--peer", "b", "in.txt", "x", "y",
        ],
        [("DB_URL", "postgres://localhost")],
    )
    .unwrap();

    assert_eq!(opts.output.as_deref(), Some("out.txt"));
    assert_eq!(opts.peer, vec!["a", "b"]);
    assert_eq!(opts.db.url, "postgres://localhost");
    assert_eq!(opts.db.retries, 3);
    assert_eq!(opts.input, "in.txt");
    assert_eq!(opts.extra, vec!["x", "y"]);
}
