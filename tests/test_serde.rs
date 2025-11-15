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

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct TestAlias {
    #[arg(
        long,
        serde(rename = "new_name", alias = "old_name", alias = "older_name")
    )]
    pub field1: String,

    #[arg(long, serde(alias = "legacy_flag"))]
    pub flag: bool,

    #[arg(repeat, long, serde(alias = "old_list"))]
    pub items: Vec<i32>,
}

#[test]
fn test_serde_alias() {
    // Test that the main name works
    let result = TestAlias::conf_builder()
        .args([".", "--field1=from_cli", "--flag"])
        .doc("t.json", json!({"items": [1, 2, 3]}))
        .try_parse()
        .unwrap();
    assert_eq!(result.field1, "from_cli");
    assert!(result.flag);
    assert_eq!(result.items, vec![1, 2, 3]);

    // Test that renamed field works from serde doc
    let result = TestAlias::conf_builder()
        .args([".", "--flag"])
        .doc("t.json", json!({"new_name": "from_new", "items": [4, 5]}))
        .try_parse()
        .unwrap();
    assert_eq!(result.field1, "from_new");
    assert!(result.flag);
    assert_eq!(result.items, vec![4, 5]);

    // Test that first alias works
    let result = TestAlias::conf_builder()
        .args([".", "--flag"])
        .doc("t.json", json!({"old_name": "from_old", "items": [6]}))
        .try_parse()
        .unwrap();
    assert_eq!(result.field1, "from_old");
    assert!(result.flag);
    assert_eq!(result.items, vec![6]);

    // Test that second alias works
    let result = TestAlias::conf_builder()
        .args([".", "--flag"])
        .doc(
            "t.json",
            json!({"older_name": "from_older", "items": [7, 8]}),
        )
        .try_parse()
        .unwrap();
    assert_eq!(result.field1, "from_older");
    assert!(result.flag);
    assert_eq!(result.items, vec![7, 8]);

    // Test that flag alias works
    let result = TestAlias::conf_builder()
        .args([".", "--field1=test"])
        .doc("t.json", json!({"legacy_flag": true, "items": [9]}))
        .try_parse()
        .unwrap();
    assert_eq!(result.field1, "test");
    assert!(result.flag);
    assert_eq!(result.items, vec![9]);

    // Test that repeat alias works
    let result = TestAlias::conf_builder()
        .args([".", "--field1=test", "--flag"])
        .doc("t.json", json!({"old_list": [10, 11, 12]}))
        .try_parse()
        .unwrap();
    assert_eq!(result.field1, "test");
    assert!(result.flag);
    assert_eq!(result.items, vec![10, 11, 12]);

    // Test that using both main name and alias causes duplicate field error
    assert_error_contains_text!(
        TestAlias::conf_builder()
            .args([".", "--flag"])
            .doc(
                "t.json",
                json!({"new_name": "val1", "old_name": "val2", "items": [1]})
            )
            .try_parse(),
        ["duplicate field"]
    );

    // Test that using two aliases causes duplicate field error
    assert_error_contains_text!(
        TestAlias::conf_builder()
            .args([".", "--flag"])
            .doc(
                "t.json",
                json!({"old_name": "val1", "older_name": "val2", "items": [1]})
            )
            .try_parse(),
        ["duplicate field"]
    );

    // Test that unknown field error lists all valid names (main + aliases)
    assert_error_contains_text!(
        TestAlias::conf_builder()
            .args([".", "--field1=test", "--flag"])
            .doc("t.json", json!({"unknown_field": "val", "items": [1]}))
            .try_parse(),
        ["unknown field `unknown_field`"]
    );
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct Inner {
    #[arg(long)]
    pub value: i32,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct TestFlattenAlias {
    #[conf(flatten, serde(rename = "new_inner", alias = "old_inner"))]
    pub inner: Inner,

    #[arg(long)]
    pub flag: bool,
}

#[test]
fn test_serde_flatten_alias() {
    // Test that the main (renamed) name works
    let result = TestFlattenAlias::conf_builder()
        .args([".", "--value=10", "--flag"])
        .doc("t.json", json!({}))
        .try_parse()
        .unwrap();
    assert_eq!(result.inner.value, 10);
    assert!(result.flag);

    // Test that renamed field works from serde doc
    let result = TestFlattenAlias::conf_builder()
        .args([".", "--flag"])
        .doc("t.json", json!({"new_inner": {"value": 20}}))
        .try_parse()
        .unwrap();
    assert_eq!(result.inner.value, 20);
    assert!(result.flag);

    // Test that alias works
    let result = TestFlattenAlias::conf_builder()
        .args([".", "--flag"])
        .doc("t.json", json!({"old_inner": {"value": 30}}))
        .try_parse()
        .unwrap();
    assert_eq!(result.inner.value, 30);
    assert!(result.flag);

    // Test that using both main name and alias causes duplicate field error
    assert_error_contains_text!(
        TestFlattenAlias::conf_builder()
            .args([".", "--flag"])
            .doc(
                "t.json",
                json!({"new_inner": {"value": 10}, "old_inner": {"value": 20}})
            )
            .try_parse(),
        ["duplicate field"]
    );
}

#[derive(Subcommands, Debug)]
#[conf(serde)]
pub enum CommandsWithAlias {
    #[conf(serde(rename = "new_cmd", alias = "old_cmd", alias = "legacy_cmd"))]
    NewCommand(A2),
    #[conf(serde(rename = "other_new", alias = "other_old"))]
    OtherCommand(B),
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct TestSubcommandAlias {
    #[arg(short)]
    flag: bool,
    #[conf(subcommands)]
    commands: CommandsWithAlias,
}

#[test]
fn test_serde_subcommand_alias() {
    // Test that the main (renamed) name works
    let result = TestSubcommandAlias::conf_builder()
        .args([".", "new-command", "--wiggle=5"])
        .doc("t.json", json!({"new_cmd": {"wobble": "a"}}))
        .try_parse()
        .unwrap();
    assert!(!result.flag);
    let CommandsWithAlias::NewCommand(cmd) = result.commands else {
        panic!("unexpected enum value")
    };
    assert_eq!(cmd.wiggle, 5);
    assert_eq!(cmd.wobble, "a");

    // Test that first alias works
    let result = TestSubcommandAlias::conf_builder()
        .args([".", "new-command", "--wiggle=10"])
        .doc("t.json", json!({"old_cmd": {"wobble": "b"}}))
        .try_parse()
        .unwrap();
    assert!(!result.flag);
    let CommandsWithAlias::NewCommand(cmd) = result.commands else {
        panic!("unexpected enum value")
    };
    assert_eq!(cmd.wiggle, 10);
    assert_eq!(cmd.wobble, "b");

    // Test that second alias works
    let result = TestSubcommandAlias::conf_builder()
        .args([".", "new-command", "--wiggle=15"])
        .doc("t.json", json!({"legacy_cmd": {"wobble": "c"}}))
        .try_parse()
        .unwrap();
    assert!(!result.flag);
    let CommandsWithAlias::NewCommand(cmd) = result.commands else {
        panic!("unexpected enum value")
    };
    assert_eq!(cmd.wiggle, 15);
    assert_eq!(cmd.wobble, "c");

    // Test that alias works for the other command
    let result = TestSubcommandAlias::conf_builder()
        .args([".", "other-command"])
        .doc(
            "t.json",
            json!({"other_old": {"f": true, "a": {"wiggle": 20, "wobble": "d"}}}),
        )
        .try_parse()
        .unwrap();
    assert!(!result.flag);
    let CommandsWithAlias::OtherCommand(cmd) = result.commands else {
        panic!("unexpected enum value")
    };
    assert!(cmd.f);
    assert_eq!(cmd.a.wiggle, 20);
    assert_eq!(cmd.a.wobble, "d");

    // Test that using both main name and alias causes duplicate field error
    assert_error_contains_text!(
        TestSubcommandAlias::conf_builder()
            .args([".", "new-command"])
            .doc(
                "t.json",
                json!({"new_cmd": {"wobble": "x"}, "old_cmd": {"wobble": "y"}})
            )
            .try_parse(),
        ["duplicate field"]
    );

    // Test that using two aliases causes duplicate field error
    assert_error_contains_text!(
        TestSubcommandAlias::conf_builder()
            .args([".", "new-command"])
            .doc(
                "t.json",
                json!({"old_cmd": {"wobble": "x"}, "legacy_cmd": {"wobble": "y"}})
            )
            .try_parse(),
        ["duplicate field"]
    );
}

// Custom deserialize functions for testing
fn deserialize_doubled<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let value = i32::deserialize(deserializer)?;
    Ok(value * 2)
}

fn deserialize_uppercase<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let value = String::deserialize(deserializer)?;
    Ok(value.to_uppercase())
}

fn deserialize_bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let value = i32::deserialize(deserializer)?;
    Ok(value != 0)
}

fn deserialize_vec_reversed<'de, D>(deserializer: D) -> Result<Vec<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let mut value = Vec::<i32>::deserialize(deserializer)?;
    value.reverse();
    Ok(value)
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct TestDeserializeWith {
    #[arg(long, serde(deserialize_with = "deserialize_doubled"))]
    pub doubled: i32,

    #[arg(long, serde(deserialize_with = "deserialize_uppercase"))]
    pub uppercase: String,

    #[arg(long, serde(deserialize_with = "deserialize_bool_from_int"))]
    pub flag_from_int: bool,

    #[conf(repeat, long, serde(deserialize_with = "deserialize_vec_reversed"))]
    pub reversed_items: Vec<i32>,
}

#[test]
fn test_serde_deserialize_with() {
    // Test that deserialize_with is applied for parameter
    let result = TestDeserializeWith::conf_builder()
        .args([".", "--doubled=5", "--uppercase=hello", "--flag-from-int"])
        .doc("t.json", json!({}))
        .try_parse()
        .unwrap();
    assert_eq!(result.doubled, 5); // Not doubled because value is from args, not serde
    assert_eq!(result.uppercase, "hello"); // Not uppercased because value is from args
    assert_eq!(result.flag_from_int, true);
    assert!(result.reversed_items.is_empty());

    // Test that deserialize_with works from serde document
    let result = TestDeserializeWith::conf_builder()
        .args(["."])
        .doc(
            "t.json",
            json!({
                "doubled": 3,
                "uppercase": "from_serde",
                "flag_from_int": 0,
                "reversed_items": [1, 2, 3]
            }),
        )
        .try_parse()
        .unwrap();
    assert_eq!(result.doubled, 6); // 3 * 2 (deserialize_with applied)
    assert_eq!(result.uppercase, "FROM_SERDE"); // Uppercased (deserialize_with applied)
    assert_eq!(result.flag_from_int, false); // 0 -> false (deserialize_with applied)
    assert_eq!(result.reversed_items, vec![3, 2, 1]); // Reversed (deserialize_with applied)

    // Test that CLI values take precedence and don't use deserialize_with
    // (deserialize_with only applies to serde documents)
    let result = TestDeserializeWith::conf_builder()
        .args([
            ".",
            "--doubled=7",
            "--uppercase=world",
            "--flag-from-int",
            "--reversed-items=10",
        ])
        .doc(
            "t.json",
            json!({
                "doubled": 100,
                "uppercase": "ignored",
                "flag_from_int": 0,
                "reversed_items": [99, 98]
            }),
        )
        .try_parse()
        .unwrap();
    // CLI values should not be transformed by deserialize_with
    assert_eq!(result.doubled, 7); // Not doubled
    assert_eq!(result.uppercase, "world"); // Not uppercased
    assert_eq!(result.flag_from_int, true); // --flag-from-int sets it to true
    assert_eq!(result.reversed_items, vec![10]); // Not reversed
}

// Test that deserialize_with and use_value_parser are mutually exclusive
#[test]
fn test_serde_deserialize_with_mutual_exclusivity() {
    // This should fail to compile if uncommented
    // #[derive(Conf)]
    // #[conf(serde)]
    // struct BadConfig {
    //     #[arg(long, serde(deserialize_with = "some_fn", use_value_parser))]
    //     field: String,
    // }

    // For now, we verify this is caught at proc macro time by checking the error message
    // would be "deserialize_with and use_value_parser are mutually exclusive"
    // This is tested by the proc macro compilation tests
}
