mod common;
use common::*;

use conf::{Conf, Subcommands};
use std::fmt::Display;
use std::marker::PhantomData;
use std::str::FromStr;

fn no_env() -> Vec<(&'static str, &'static str)> {
    vec![]
}

#[derive(Debug, PartialEq)]
struct Counter(u32);

impl Default for Counter {
    fn default() -> Self {
        Self(7)
    }
}

#[derive(Conf, Debug)]
struct SkipConfig {
    /// A real option
    #[arg(long)]
    name: String,
    /// Not an option
    #[arg(skip)]
    counter: Counter,
    #[conf(skip = vec![1, 2])]
    list: Vec<u32>,
    // Without skip, a bool would be a flag and an Option an optional parameter
    #[arg(skip)]
    verbose: bool,
    #[arg(skip)]
    maybe: Option<u32>,
}

#[test]
fn test_skip() {
    let s = SkipConfig::try_parse_from(["prog", "--name", "x"], [("COUNTER", "1")]).unwrap();
    assert_eq!(s.name, "x");
    assert_eq!(s.counter, Counter(7));
    assert_eq!(s.list, vec![1, 2]);
    assert!(!s.verbose);
    assert_eq!(s.maybe, None);

    assert_error_contains_text!(
        SkipConfig::try_parse_from(["prog", "--name", "x", "--counter", "1"], no_env()),
        ["unexpected argument '--counter'"]
    );
}

#[test]
fn test_skip_not_in_help() {
    let help = SkipConfig::try_parse_from(["prog", "--help"], no_env())
        .unwrap_err()
        .to_string();
    assert!(help.contains("A real option"), "{help}");
    assert!(!help.contains("counter"), "{help}");
    assert!(!help.contains("Not an option"), "{help}");
    assert!(!help.contains("list"), "{help}");
    assert!(!help.contains("verbose"), "{help}");
    assert!(!help.contains("maybe"), "{help}");
}

// The motivating case: a generic struct that holds a type param only in a PhantomData.
// `skip = expr` can also use the struct's generic params.
#[derive(Conf, Debug)]
struct Tagged<Tag, const N: usize> {
    #[arg(long)]
    value: u32,
    #[arg(skip)]
    _tag: PhantomData<Tag>,
    #[arg(skip = N * 2)]
    doubled: usize,
}

#[derive(Debug)]
struct NotParseable;

#[test]
fn test_skip_generic() {
    let s = Tagged::<NotParseable, 3>::try_parse_from(["prog", "--value", "1"], no_env()).unwrap();
    assert_eq!(s.value, 1);
    assert_eq!(s.doubled, 6);
}

#[derive(Conf, Debug)]
struct WithParser<T>
where
    T: FromStr,
    T::Err: Display,
{
    #[arg(long)]
    value: T,
    #[arg(skip = value_twice::<T>())]
    twice: Option<(T, T)>,
}

fn value_twice<T>() -> Option<(T, T)> {
    None
}

#[test]
fn test_skip_expr_with_generic_fn() {
    let s = WithParser::<u8>::try_parse_from(["prog", "--value", "4"], no_env()).unwrap();
    assert_eq!(s.value, 4);
    assert!(s.twice.is_none());
}

// A skipped field in a named-field subcommand variant
#[derive(Subcommands, Debug)]
enum Command {
    Run {
        #[arg(long)]
        jobs: u32,
        #[arg(skip = 5)]
        retries: u32,
    },
}

#[derive(Conf, Debug)]
struct Cli {
    #[arg(subcommands)]
    command: Command,
}

#[test]
fn test_skip_in_subcommand_variant() {
    let cli = Cli::try_parse_from(["prog", "run", "--jobs", "2"], no_env()).unwrap();
    let Command::Run { jobs, retries } = cli.command;
    assert_eq!(jobs, 2);
    assert_eq!(retries, 5);
}

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Conf, Debug)]
    #[conf(serde)]
    struct SerdeSkipConfig<T>
    where
        T: FromStr + for<'de> Deserialize<'de>,
        T::Err: Display,
    {
        #[arg(long)]
        value: T,
        #[arg(skip)]
        _marker: PhantomData<T>,
        #[arg(skip = 3)]
        hidden: u32,
    }

    #[test]
    fn test_skip_serde() {
        let s = SerdeSkipConfig::<u32>::conf_builder()
            .args(["prog"])
            .env(no_env())
            .doc("doc", json!({ "value": 2 }))
            .try_parse()
            .unwrap();
        assert_eq!(s.value, 2);
        assert_eq!(s.hidden, 3);

        // Like serde(skip), a skipped field isn't a known key in the document
        assert_error_contains_text!(
            SerdeSkipConfig::<u32>::conf_builder()
                .args(["prog"])
                .env(no_env())
                .doc("doc", json!({ "value": 2, "hidden": 4 }))
                .try_parse(),
            ["unknown field `hidden`, expected `value`"]
        );
    }
}

// The example from REFERENCE_derive_conf.md
#[derive(Conf)]
pub struct MyConfig<Tag> {
    #[conf(long)]
    pub value: u32,

    // Initialized with Default::default()
    #[conf(skip)]
    pub _tag: PhantomData<Tag>,

    // Initialized with this expression
    #[conf(skip = 3)]
    pub retries: u32,
}

#[test]
fn test_reference_example() {
    let s = MyConfig::<NotParseable>::try_parse_from(["prog", "--value", "1"], no_env()).unwrap();
    assert_eq!(s.value, 1);
    assert_eq!(s.retries, 3);
}

// `Self` can't be used in a skip expression, but the struct's name can
#[derive(Conf, Debug)]
struct NamedConst {
    #[arg(skip = NamedConst::DEFAULT_RETRIES)]
    retries: u32,
    #[arg(skip = String::from("guest"))]
    user: String,
}

impl NamedConst {
    const DEFAULT_RETRIES: u32 = 4;
}

#[test]
fn test_skip_named_const() {
    let s = NamedConst::try_parse_from(["prog"], no_env()).unwrap();
    assert_eq!(s.retries, 4);
    assert_eq!(s.user, "guest");
}
