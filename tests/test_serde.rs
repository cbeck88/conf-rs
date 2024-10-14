#![cfg(feature = "serde")]

mod common;
use common::*;

use conf::Conf;
use serde_json::json;

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct A {
    #[arg(long, env)]
    pub wiggle: i16,
    #[arg(long, env)]
    pub wobble: String,
    #[arg(long, env)]
    pub bobble: Option<i16>,
}

#[test]
fn test_basic_serde() {
    let result = A::conf_builder()
        .args([".", "--wiggle=8"])
        .env([("WOBBLE", "xxx")])
        .doc(
            "test_doc",
            json! ({
              "bobble": 9
            }),
        )
        .try_parse()
        .unwrap();
    assert_eq!(result.wiggle, 8);
    assert_eq!(result.wobble, "xxx");
    assert_eq!(result.bobble, Some(9));

    let result = A::conf_builder()
        .args([".", "--wiggle=8", "--bobble", "7"])
        .env([("WOBBLE", "xxx")])
        .doc(
            "test_doc",
            json! ({
              "bobble": 9
            }),
        )
        .try_parse()
        .unwrap();
    assert_eq!(result.wiggle, 8);
    assert_eq!(result.wobble, "xxx");
    assert_eq!(result.bobble, Some(7));

    assert_error_contains_text!(
        A::conf_builder()
            .args([".", "--bobble", "7"])
            .env([("WOBBLE", "xxx")])
            .doc(
                "test_doc",
                json! ({
                  "bobble": 9
                }),
            )
            .try_parse(),
        ["env 'WIGGLE', or '--wiggle', must be provided"]
    );

    let result = A::conf_builder()
        .args([".", "--bobble", "7"])
        .env([("WOBBLE", "xxx")])
        .doc(
            "test_doc",
            json! ({
              "bobble": 9,
              "wiggle": -2
            }),
        )
        .try_parse()
        .unwrap();
    assert_eq!(result.wiggle, -2);
    assert_eq!(result.wobble, "xxx");
    assert_eq!(result.bobble, Some(7));

    let result = A::conf_builder()
        .args([".", "--bobble", "7"])
        .env([("WOBBLE", "xxx")])
        .doc(
            "test_doc",
            json! ({
              "bobble": 9,
              "wiggle": -2,
              "wobble": "yyy"
            }),
        )
        .try_parse()
        .unwrap();
    assert_eq!(result.wiggle, -2);
    assert_eq!(result.wobble, "xxx");
    assert_eq!(result.bobble, Some(7));

    let result = A::conf_builder()
        .args([".", "--bobble", "7"])
        .env([("WOBBLY", "xxx")])
        .doc(
            "test_doc",
            json! ({
              "bobble": 9,
              "wiggle": -2,
              "wobble": "yyy"
            }),
        )
        .try_parse()
        .unwrap();
    assert_eq!(result.wiggle, -2);
    assert_eq!(result.wobble, "yyy");
    assert_eq!(result.bobble, Some(7));

    assert_error_contains_text!(
        A::conf_builder()
            .args([".", "--bobble", "7", "--bobble=4"])
            .env([("WOBBLY", "xxx")])
            .doc(
                "test_doc",
                json! ({
                  "bobble": 9,
                  "wiggle": -2,
                  "wobble": "yyy",
                }),
            )
            .try_parse(),
        ["the argument '--bobble <bobble>' cannot be used multiple times"]
    );

    assert_error_contains_text!(
        A::conf_builder()
            .args([".", "--bobble", "7"])
            .env([("WOBBLY", "xxx")])
            .doc(
                "test_doc",
                json! ({
                  "bobble": 9,
                  "wiggle": -2,
                  "wobble": "yyy",
                  "wobbly": "zzz"
                }),
            )
            .try_parse(),
        ["Parsing test_doc (@ A): unknown field `wobbly`"]
    );

    assert_error_contains_text!(
        A::conf_builder()
            .args([".", "--bobble", "x", "--wiggle=o"])
            .env([("WOBBLY", "xxx")])
            .doc(
                "test_doc",
                json! ({
                  "bobble": 9,
                  "wiggle": -2,
                  "wobble": "yyy",
                  "wobbly": "zzz",
                  "wubbly": "qqq",
                }),
            )
            .try_parse(),
        [
            "when parsing '--wiggle' value 'o': invalid digit found in string",
            "when parsing '--bobble' value 'x': invalid digit found in string",
            "Parsing test_doc (@ A): unknown field `wobbly`",
            "Parsing test_doc (@ A): unknown field `wubbly`",
        ]
    );
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct B {
    #[conf(flatten)]
    a: A,
    #[arg(short)]
    f: bool,
}

#[test]
fn test_serde_nested() {
    assert_error_contains_text!(
        B::conf_builder()
            .args([".", "--wiggle=8"])
            .env([("WOBBLE", "xxx")])
            .doc(
                "test_doc",
                json! ({
                  "bobble": 9
                }),
            )
            .try_parse(),
        ["Parsing test_doc (@ B): unknown field `bobble`"]
    );

    let result = B::conf_builder()
        .args([".", "--wiggle=8"])
        .env([("WOBBLE", "xxx")])
        .doc(
            "test_doc",
            json! ({
              "a": {
                "bobble": 9
              }
            }),
        )
        .try_parse()
        .unwrap();
    assert!(!result.f);
    assert_eq!(result.a.wiggle, 8);
    assert_eq!(result.a.wobble, "xxx");
    assert_eq!(result.a.bobble, Some(9));

    let result = B::conf_builder()
        .args([".", "--wiggle=8"])
        .env([("WOBBLE", "xxx")])
        .doc(
            "test_doc",
            json! ({
              "a": {
                "bobble": 9
              },
              "f": true
            }),
        )
        .try_parse()
        .unwrap();
    assert!(result.f);
    assert_eq!(result.a.wiggle, 8);
    assert_eq!(result.a.wobble, "xxx");
    assert_eq!(result.a.bobble, Some(9));

    assert_error_contains_text!(
        B::conf_builder()
            .args([".", "--wiggle=q"])
            .env([("WOBBLE", "xxx")])
            .doc(
                "test_doc3",
                json! ({
                  "a": {
                    "bobble": "xxx"
                  },
                  "f": 7,
                  "n": "q"
                }),
            )
            .try_parse(),
        [
            "when parsing '--wiggle' value 'q': invalid digit found in string",
            "Parsing test_doc3 (@ bobble): invalid type: string \"xxx\", expected i16",
            "Parsing test_doc3 (@ f): invalid type: integer `7`, expected a boolean",
            "Parsing test_doc3 (@ B): unknown field `n`, expected `a` or `f`"
        ]
    );
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct C {
    #[arg(repeat, long, env)]
    out: Vec<String>,
    #[arg(repeat, long, env)]
    p: Vec<i64>,
}

#[test]
fn test_serde_repeat() {
    let result = C::conf_builder()
        .args([
            ".", "--out", "asdf", "--p", "1", "--out", "jkl", "--p", "-1",
        ])
        .doc("test_doc", json!({}))
        .try_parse()
        .unwrap();
    assert_eq!(result.out, vec!["asdf", "jkl"]);
    assert_eq!(result.p, vec![1, -1]);

    let result = C::conf_builder()
        .args([".", "--out", "asdf", "--out", "jkl"])
        .doc("test_doc", json!({ "p": [1, -1]}))
        .try_parse()
        .unwrap();
    assert_eq!(result.out, vec!["asdf", "jkl"]);
    assert_eq!(result.p, vec![1, -1]);

    let result = C::conf_builder()
        .args([".", "--out", "asdf", "--p", "99", "--out", "jkl"])
        .doc("test_doc", json!({ "p": [1, -1]}))
        .try_parse()
        .unwrap();
    assert_eq!(result.out, vec!["asdf", "jkl"]);
    assert_eq!(result.p, vec![99]);

    let result = C::conf_builder()
        .args(["."])
        .doc("test_doc", json!({ "p": [1, -1], "out": ["asdf", "jkl"]}))
        .try_parse()
        .unwrap();
    assert_eq!(result.out, vec!["asdf", "jkl"]);
    assert_eq!(result.p, vec![1, -1]);

    assert_error_contains_text!(
        C::conf_builder()
            .args(["."])
            .doc("test_doc", json!({ "out": [1, -1], "p": ["asdf", "jkl"]}))
            .try_parse(),
        [
            "Parsing test_doc (@ out): invalid type: integer `1`, expected a string",
            "Parsing test_doc (@ p): invalid type: string \"asdf\", expected i64"
        ]
    );
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct A2 {
    #[arg(long, env)]
    pub wiggle: i16,
    #[arg(long, env)]
    pub wobble: String,
    #[arg(long, env, serde(use_value_parser))]
    pub bobble: Option<i16>,
    #[arg(repeat, long, env, serde(use_value_parser))]
    pub out: Vec<u64>,
}

#[test]
fn test_serde_use_value_parser() {
    let result = A2::conf_builder()
        .args([".", "--wiggle=8"])
        .env([("WOBBLE", "xxx")])
        .doc(
            "test_doc",
            json! ({
              "bobble": "9"
            }),
        )
        .try_parse()
        .unwrap();
    assert_eq!(result.wiggle, 8);
    assert_eq!(result.wobble, "xxx");
    assert_eq!(result.bobble, Some(9));
    assert!(result.out.is_empty());

    assert_error_contains_text!(
        A2::conf_builder()
            .args([".", "--wiggle=8"])
            .env([("WOBBLE", "xxx")])
            .doc(
                "test_doc",
                json! ({
                  "bobble": 9
                }),
            )
            .try_parse(),
        ["Parsing test_doc (@ bobble): invalid type: integer `9`, expected a string"]
    );

    let result = A2::conf_builder()
        .args([".", "--wiggle=8"])
        .env([("WOBBLE", "xxx")])
        .doc(
            "test_doc",
            json! ({
              "bobble": "9",
              "out": ["99", "44", "77"],
            }),
        )
        .try_parse()
        .unwrap();
    assert_eq!(result.wiggle, 8);
    assert_eq!(result.wobble, "xxx");
    assert_eq!(result.bobble, Some(9));
    assert_eq!(result.out, vec![99, 44, 77]);

    assert_error_contains_text!(
        A2::conf_builder()
            .args([".", "--wiggle=8"])
            .env([("WOBBLE", "xxx")])
            .doc(
                "test_doc",
                json! ({
                  "bobble": "9",
                  "out": [99, 44, 77],
                }),
            )
            .try_parse(),
        ["Parsing test_doc (@ out): invalid type: integer `99`, expected a string"]
    );
}

// Custom data that implements FromStr but not serde::Deserialize
#[derive(Debug)]
pub struct CustomData {
    val1: i64,
    val2: i64,
}

use std::str::FromStr;
impl FromStr for CustomData {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pieces = s.split(':').collect::<Vec<&str>>();
        if pieces.len() != 2 {
            return Err("Expected one ':'");
        }
        Ok(Self {
            val1: FromStr::from_str(pieces[0]).map_err(|_| "Bad first number")?,
            val2: FromStr::from_str(pieces[1]).map_err(|_| "Bad second number")?,
        })
    }
}

// A struct that implements Conf but not ConfSerde
#[derive(Conf, Debug)]
pub struct NotSerde {
    #[arg(long, env)]
    pub my_param: String,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct TestSerdeSkip {
    #[arg(short, env, serde(skip))]
    pub f: bool,
    #[arg(long, env, serde(skip))]
    pub pair: CustomData,
    #[arg(repeat, long, env, serde(skip))]
    pub pairs: Vec<CustomData>,
    #[conf(flatten, serde(skip))]
    pub not_serde: NotSerde,
    #[arg(long, env)]
    pub val: u64,
}

#[test]
fn test_serde_skip() {
    let result = TestSerdeSkip::conf_builder()
        .args([".", "--pair=2:3"])
        .env([("MY_PARAM", "Asdf"), ("VAL", "2")])
        .try_parse()
        .unwrap();

    assert!(!result.f);
    assert_eq!(result.pair.val1, 2);
    assert_eq!(result.pair.val2, 3);
    assert!(result.pairs.is_empty());
    assert_eq!(result.not_serde.my_param, "Asdf");
    assert_eq!(result.val, 2);

    let result = TestSerdeSkip::conf_builder()
        .args([".", "--pair=2:3"])
        .env([("MY_PARAM", "Asdf")])
        .doc("test", json!({"val": 2}))
        .try_parse()
        .unwrap();

    assert!(!result.f);
    assert_eq!(result.pair.val1, 2);
    assert_eq!(result.pair.val2, 3);
    assert!(result.pairs.is_empty());
    assert_eq!(result.not_serde.my_param, "Asdf");
    assert_eq!(result.val, 2);

    assert_error_contains_text!(
        TestSerdeSkip::conf_builder()
            .args([".", "--pair=2:3"])
            .env([("MY_PARAM", "Asdf")])
            .doc("test", json!({"f": true, "val": 2}))
            .try_parse(),
        ["Parsing test (@ TestSerdeSkip): unknown field `f`, expected `val`"]
    );

    assert_error_contains_text!(
        TestSerdeSkip::conf_builder()
            .args([".", "--pair=2:3"])
            .env([("MY_PARAM", "Asdf")])
            .doc(
                "test",
                json!({"not_serde": { "my_param": "Foo" }, "val": 2})
            )
            .try_parse(),
        ["Parsing test (@ TestSerdeSkip): unknown field `not_serde`, expected `val`"]
    );

    assert_error_contains_text!(
        TestSerdeSkip::conf_builder()
            .args([".", "--pair=2:3"])
            .env([("MY_PARAM", "Asdf")])
            .doc("test", json!({"pair": "2:3", "val": 2}))
            .try_parse(),
        ["Parsing test (@ TestSerdeSkip): unknown field `pair`, expected `val`"]
    );

    assert_error_contains_text!(
        TestSerdeSkip::conf_builder()
            .args([".", "--pair=2:3"])
            .env([("MY_PARAM", "Asdf")])
            .doc("test", json!({"pairs": ["2:3"], "val": 2}))
            .try_parse(),
        ["Parsing test (@ TestSerdeSkip): unknown field `pairs`, expected `val`"]
    );

    let result = TestSerdeSkip::conf_builder()
        .args([".", "--pair=2:3"])
        .env([("MY_PARAM", "Asdf"), ("PAIRS", "1:2,3:4,5:6")])
        .doc("test", json!({"val": 2}))
        .try_parse()
        .unwrap();

    assert!(!result.f);
    assert_eq!(result.pair.val1, 2);
    assert_eq!(result.pair.val2, 3);
    assert_eq!(result.pairs.len(), 3);
    assert_eq!(result.pairs[0].val1, 1);
    assert_eq!(result.pairs[0].val2, 2);
    assert_eq!(result.pairs[1].val1, 3);
    assert_eq!(result.pairs[1].val2, 4);
    assert_eq!(result.pairs[2].val1, 5);
    assert_eq!(result.pairs[2].val2, 6);
    assert_eq!(result.not_serde.my_param, "Asdf");
    assert_eq!(result.val, 2);
}

#[derive(Conf)]
#[conf(serde)]
pub struct E {
    #[arg(long, default_value = "def", serde(rename = "p"))]
    pub param: String,
}

#[derive(Conf)]
#[conf(serde)]
pub struct D {
    #[arg(long, serde(rename = "f"))]
    pub force: bool,
    #[conf(flatten, serde(rename = "a"))]
    pub a2: A2,
    #[conf(flatten, prefix, serde(rename = "a2"))]
    pub b: B,
    #[conf(long, serde(rename = "p"))]
    pub p: String,
    #[conf(repeat, long, env = "qs", serde(rename = "qs"))]
    pub q: Vec<String>,
    #[conf(flatten)]
    pub e: E,
}

#[test]
fn test_serde_rename() {
    let result = D::conf_builder()
        .args([
            ".",
            "--wiggle=4",
            "--wobble=9",
            "--b-wiggle=10",
            "--b-wobble=14",
            "--p=xyz",
        ])
        .env::<&str, &str>([])
        .try_parse()
        .unwrap();

    assert!(!result.force);
    assert_eq!(result.a2.wiggle, 4);
    assert_eq!(result.a2.wobble, "9");
    assert_eq!(result.a2.bobble, None);
    assert!(!result.b.f);
    assert_eq!(result.b.a.wiggle, 10);
    assert_eq!(result.b.a.wobble, "14");
    assert_eq!(result.b.a.bobble, None);
    assert_eq!(result.p, "xyz");
    assert!(result.q.is_empty());
    assert_eq!(result.e.param, "def");

    let result = D::conf_builder()
        .args([
            ".",
            "--wiggle=4",
            "--wobble=9",
            "--b-wiggle=10",
            "--b-wobble=14",
            "--p=xyz",
        ])
        .env::<&str, &str>([])
        .doc("t.json", json!({ "f": true, "a2": { "f": true }}))
        .try_parse()
        .unwrap();

    assert!(result.force);
    assert_eq!(result.a2.wiggle, 4);
    assert_eq!(result.a2.wobble, "9");
    assert_eq!(result.a2.bobble, None);
    assert!(result.b.f);
    assert_eq!(result.b.a.wiggle, 10);
    assert_eq!(result.b.a.wobble, "14");
    assert_eq!(result.b.a.bobble, None);
    assert_eq!(result.p, "xyz");
    assert!(result.q.is_empty());
    assert_eq!(result.e.param, "def");

    let result = D::conf_builder()
        .args([".", "--wiggle=4", "--wobble=9", "--b-wiggle=10", "--b-wobble=14", "--p=xyz"])
        .env::<&str, &str>([])
        .doc("t.json", json!({ "f": true, "a2": { "f": true, "a": {"wiggle": 7, "bobble": -8 }}, "e": { "p": "shadow" }}))
        .try_parse()
        .unwrap();

    assert!(result.force);
    assert_eq!(result.a2.wiggle, 4);
    assert_eq!(result.a2.wobble, "9");
    assert_eq!(result.a2.bobble, None);
    assert!(result.b.f);
    assert_eq!(result.b.a.wiggle, 10);
    assert_eq!(result.b.a.wobble, "14");
    assert_eq!(result.b.a.bobble, Some(-8));
    assert_eq!(result.p, "xyz");
    assert!(result.q.is_empty());
    assert_eq!(result.e.param, "shadow");
}

use conf::Subcommands;
#[derive(Subcommands, Debug)]
#[conf(serde)]
pub enum Commands {
    A(A2),
    B(B),
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct S {
    #[arg(short)]
    f: bool,
    #[conf(subcommands)]
    commands: Commands,
}

#[test]
fn test_subcommands_serde() {
    let result = S::conf_builder()
        .args([".", "a", "--wiggle=4"])
        .env([("WOBBLE", "x")])
        .doc("t.json", json!({}))
        .try_parse()
        .unwrap();

    assert!(!result.f);
    let Commands::A(a2) = result.commands else {
        panic!("unexpected enum value")
    };
    assert_eq!(a2.wiggle, 4);
    assert_eq!(a2.wobble, "x");
    assert_eq!(a2.bobble, None);

    assert_error_contains_text!(
        S::conf_builder()
            .args(["."])
            .env([("WOBBLE", "x")])
            .doc("t.json", json!({}))
            .try_parse(),
        ["Missing required subcommand"]
    );

    let result = S::conf_builder()
        .args([".", "a"])
        .env([("LANG", "C")])
        .doc("t.json", json!({"a": { "wiggle": 4, "wobble": "x"}}))
        .try_parse()
        .unwrap();

    assert!(!result.f);
    let Commands::A(a2) = result.commands else {
        panic!("unexpected enum value")
    };
    assert_eq!(a2.wiggle, 4);
    assert_eq!(a2.wobble, "x");
    assert_eq!(a2.bobble, None);

    let result = S::conf_builder()
        .args([".", "a"])
        .env([("LANG", "C")])
        .doc("t.json", json!({"a": { "wiggle": 4, "wobble": "x"}, "b": {"f": true, "a": {"wiggle": 7, "wobble": "y"}}}))
        .try_parse()
        .unwrap();

    assert!(!result.f);
    let Commands::A(a2) = result.commands else {
        panic!("unexpected enum value")
    };
    assert_eq!(a2.wiggle, 4);
    assert_eq!(a2.wobble, "x");
    assert_eq!(a2.bobble, None);

    let result = S::conf_builder()
        .args([".", "b"])
        .env([("LANG", "C")])
        .doc("t.json", json!({"a": { "wiggle": 4, "wobble": "x"}, "b": {"f": true, "a": {"wiggle": 7, "wobble": "y"}}}))
        .try_parse()
        .unwrap();

    assert!(!result.f);
    let Commands::B(b) = result.commands else {
        panic!("unexpected enum value")
    };
    assert!(b.f);
    assert_eq!(b.a.wiggle, 7);
    assert_eq!(b.a.wobble, "y");
    assert_eq!(b.a.bobble, None);
}
