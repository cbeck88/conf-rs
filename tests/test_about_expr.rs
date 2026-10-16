// `about = <expr>` on structs and subcommand variants, e.g. for translated help

use conf::{Conf, Subcommands};
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
        ("en", "app.about") => "Frobnicates things".into(),
        ("fr", "app.about") => "Frobnique des choses".into(),
        ("en", "run.about") => "Run the thing\n\nRuns it for a long time.".into(),
        ("fr", "run.about") => "Lancer la chose\n\nLa lance longtemps.".into(),
        ("en", "stop.about") => "Stop the thing".into(),
        ("en", "edit.about") => "Edit the thing".into(),
        ("en", "inspect.about") => "Inspect the thing\n\nShows everything about it.".into(),
        _ => format!("<missing {key}>"),
    }
}

const STATIC_ABOUT: &str = "From a const";

fn help_of<C: Conf>(args: &[&str]) -> String {
    C::try_parse_from(args.iter().copied(), no_env())
        .err()
        .expect("--help should return an error")
        .to_string()
}

/// This doc comment is for developers, the about expression is shown instead
#[derive(Conf, Debug)]
#[conf(about = tr("app.about"))]
struct TranslatedApp {}

#[test]
fn struct_about_expr_is_evaluated_at_runtime() {
    LANG.with(|l| l.set("en"));
    let help = help_of::<TranslatedApp>(&["prog", "--help"]);
    assert!(help.starts_with("Frobnicates things\n"), "{help}");
    assert!(!help.contains("developers"), "{help}");

    // Changing the "locale" changes the help, so the expression isn't evaluated at compile time
    LANG.with(|l| l.set("fr"));
    let help = help_of::<TranslatedApp>(&["prog", "--help"]);
    assert!(help.starts_with("Frobnique des choses\n"), "{help}");
    LANG.with(|l| l.set("en"));
}

#[test]
fn struct_about_expr_in_parser_config() {
    LANG.with(|l| l.set("en"));
    let parser_config = TranslatedApp::get_parser_config().unwrap();
    assert_eq!(parser_config.about.as_deref(), Some("Frobnicates things"));
}

#[test]
fn struct_about_accepts_str_and_string_expressions() {
    #[derive(Conf, Debug)]
    #[conf(about = STATIC_ABOUT)]
    struct FromConst {}

    #[derive(Conf, Debug)]
    #[conf(about = format!("{} v{}", "Frob", 2))]
    struct FromFormat {}

    assert_eq!(
        FromConst::get_parser_config().unwrap().about.as_deref(),
        Some("From a const")
    );
    assert_eq!(
        FromFormat::get_parser_config().unwrap().about.as_deref(),
        Some("Frob v2")
    );
}

#[test]
fn struct_about_literal_and_doc_string_still_work() {
    /// From the doc string
    #[derive(Conf, Debug)]
    struct FromDoc {}

    #[derive(Conf, Debug)]
    #[conf(about = "From a literal")]
    struct FromLiteral {}

    #[derive(Conf, Debug)]
    struct NoAbout {}

    assert_eq!(
        FromDoc::get_parser_config().unwrap().about.as_deref(),
        Some("From the doc string")
    );
    assert_eq!(
        FromLiteral::get_parser_config().unwrap().about.as_deref(),
        Some("From a literal")
    );
    assert!(NoAbout::get_parser_config().unwrap().about.is_none());
}

#[derive(Conf, Debug)]
struct RunArgs {
    #[conf(long)]
    fast: bool,
}

#[derive(Subcommands, Debug)]
enum Command {
    /// Doc comment for developers
    #[conf(about = tr("run.about"))]
    Run(RunArgs),
    #[conf(about = tr("stop.about"))]
    Stop,
    /// Doc comment on a named-field variant, also for developers
    #[conf(about = tr("edit.about"))]
    Edit {
        #[conf(long)]
        file: String,
    },
    // Named fields, a multi-line about, and no doc comment
    #[conf(about = tr("inspect.about"))]
    Inspect {
        #[conf(long)]
        verbose: bool,
    },
    /// Uses the doc string
    Status,
}

#[derive(Conf, Debug)]
struct Cli {
    #[conf(subcommands)]
    command: Command,
}

#[test]
fn variant_about_expr_in_command_list() {
    LANG.with(|l| l.set("en"));
    let help = help_of::<Cli>(&["prog", "--help"]);
    let expected = &"
Usage: prog [COMMAND]

Commands:
  run      Run the thing
  stop     Stop the thing
  edit     Edit the thing
  inspect  Inspect the thing
  status   Uses the doc string
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
"[1..];
    assert_multiline_eq!(&help, expected);
}

#[test]
fn variant_about_expr_in_subcommand_help() {
    LANG.with(|l| l.set("en"));
    let help = help_of::<Cli>(&["prog", "run", "--help"]);
    assert!(
        help.starts_with("Run the thing\n\nRuns it for a long time.\n"),
        "{help}"
    );
    assert!(!help.contains("developers"), "{help}");

    LANG.with(|l| l.set("fr"));
    let help = help_of::<Cli>(&["prog", "run", "--help"]);
    assert!(
        help.starts_with("Lancer la chose\n\nLa lance longtemps.\n"),
        "{help}"
    );
    LANG.with(|l| l.set("en"));

    // For a named-field variant, `about` is handled by the variant, and not passed through to
    // the generated struct, so it overrides the doc comment here too
    let help = help_of::<Cli>(&["prog", "edit", "--help"]);
    assert!(help.starts_with("Edit the thing\n"), "{help}");
    assert!(!help.contains("developers"), "{help}");

    // The command list shows only the first line (see variant_about_expr_in_command_list),
    // and the subcommand's own help shows all of it
    let help = help_of::<Cli>(&["prog", "inspect", "--help"]);
    assert!(
        help.starts_with("Inspect the thing\n\nShows everything about it.\n"),
        "{help}"
    );
}

#[test]
fn variant_about_expr_still_parses() {
    let cli = Cli::try_parse_from(["prog", "edit", "--file", "a.txt"], no_env()).unwrap();
    assert!(matches!(cli.command, Command::Edit { file } if file == "a.txt"));

    let cli = Cli::try_parse_from(["prog", "inspect", "--verbose"], no_env()).unwrap();
    assert!(matches!(cli.command, Command::Inspect { verbose: true }));

    let cli = Cli::try_parse_from(["prog", "run", "--fast"], no_env()).unwrap();
    assert!(matches!(cli.command, Command::Run(RunArgs { fast: true })));
}
