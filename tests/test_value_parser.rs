mod common;
use common::*;

use conf::Conf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MyJson {
    a: String,
    b: i64,
}

#[derive(Conf, Debug)]
struct TestValueParserJson {
    // json argument, serde_json::from_str just works
    #[conf(long, env, value_parser = serde_json::from_str)]
    my_json: MyJson,

    // using value_parser with repeat applies the value parser to one item at a time
    #[conf(repeat, long="fallback_json", value_parser = serde_json::from_str)]
    fallback_jsons: Vec<MyJson>,
}

#[test]
fn test_value_parser_json() {
    assert_error_contains_text!(
        TestValueParserJson::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["required value was not provided", "env 'MY_JSON'"],
        not["--fallback"]
    );

    let result = TestValueParserJson::try_parse_from::<&str, &str, &str>(
        vec![".", "--my-json={\"a\":\"foo\", \"b\": 9}"],
        vec![],
    )
    .unwrap();

    assert_eq!(result.my_json.a, "foo");
    assert_eq!(result.my_json.b, 9);
    assert_eq!(result.fallback_jsons.len(), 0);

    // unknown fields causes an error (with serde(deny_unknown_fields))
    assert_error_contains_text!(
        TestValueParserJson::try_parse_from::<&str, &str, &str>(
            vec![".", "--my-json={\"a\":\"foo\", \"b\": 9, \"c\": 7}"],
            vec![]
        ),
        [
            "Invalid value",
            "when parsing '--my-json' value '{\"a\":\"foo\"",
            "unknown field `c`"
        ]
    );

    // Specifying json for a repeat parameter works as expected
    let result = TestValueParserJson::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--my-json={\"a\":\"foo\", \"b\": 9}",
            "--fallback_json",
            "{\"a\": \"bar\", \"b\": 42}",
            "--fallback_json={\"a\": \"baz\", \"b\": 19}",
        ],
        vec![],
    )
    .unwrap();

    assert_eq!(result.my_json.a, "foo");
    assert_eq!(result.my_json.b, 9);
    assert_eq!(result.fallback_jsons.len(), 2);
    assert_eq!(result.fallback_jsons[0].a, "bar");
    assert_eq!(result.fallback_jsons[0].b, 42);
    assert_eq!(result.fallback_jsons[1].a, "baz");
    assert_eq!(result.fallback_jsons[1].b, 19);

    // If one repeat parameter is invalid, then parsing fails
    assert_error_contains_text!(
        TestValueParserJson::try_parse_from::<&str, &str, &str>(
            vec![
                ".",
                "--my-json={\"a\":\"foo\", \"b\": 9}",
                "--fallback_json",
                "{\"a\": \"bar\", \"b\": 42}",
                "--fallback_json={\"a\": \"baz\", \"b\": 19, \"d\": 11}"
            ],
            vec![]
        ),
        [
            "Invalid value",
            "when parsing '--fallback_json' value '{\"a\": \"baz\"",
            "unknown field `d`"
        ],
        not["\"foo\""]
    );
}

#[derive(Conf, Debug)]
struct TestValueParserJson2 {
    // parse a vec of jsons, without repeat keyword, but still using serde_json::from_str as value parser
    #[conf(long, env, value_parser = serde_json::from_str)]
    jsons: Vec<MyJson>,
}

#[test]
fn test_value_parser_json2() {
    assert_error_contains_text!(
        TestValueParserJson2::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["required value was not provided", "env 'JSONS'"]
    );

    let result = TestValueParserJson2::try_parse_from::<&str, &str, &str>(
        vec![".", "--jsons=[{\"a\":\"foo\", \"b\": 9}]"],
        vec![],
    )
    .unwrap();

    assert_eq!(result.jsons.len(), 1);
    assert_eq!(result.jsons[0].a, "foo");
    assert_eq!(result.jsons[0].b, 9);

    let result = TestValueParserJson2::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![(
            "JSONS",
            "[{\"a\":\"foo\", \"b\": 9}, {\"a\":\"bar\", \"b\": 42}]",
        )],
    )
    .unwrap();

    assert_eq!(result.jsons.len(), 2);
    assert_eq!(result.jsons[0].a, "foo");
    assert_eq!(result.jsons[0].b, 9);
    assert_eq!(result.jsons[1].a, "bar");
    assert_eq!(result.jsons[1].b, 42);
}

fn strict_bool(arg: &str) -> Result<bool, &'static str> {
    if arg == "1" {
        Ok(true)
    } else if arg == "0" {
        Ok(false)
    } else {
        Err("not explicit enough")
    }
}

#[derive(Conf, Debug)]
struct TestValueParserBool {
    // this is a flag
    #[conf(short, long, env)]
    flag: bool,
    // this seems like a flag, but it's actually a required parameter whose value is bool
    // the parameter keyword is needed to override the automatic selection of flag here
    #[conf(parameter, short, long, env, value_parser = strict_bool)]
    strict: bool,
}

#[test]
fn test_value_parser_bool() {
    assert_error_contains_text!(
        TestValueParserBool::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        [
            "required value was not provided",
            "env 'STRICT'",
            "'--strict'"
        ]
    );

    let result =
        TestValueParserBool::try_parse_from::<&str, &str, &str>(vec![".", "--strict=1"], vec![])
            .unwrap();

    assert!(!result.flag);
    assert!(result.strict);

    let result =
        TestValueParserBool::try_parse_from::<&str, &str, &str>(vec![".", "--strict=0"], vec![])
            .unwrap();

    assert!(!result.flag);
    assert!(!result.strict);

    assert_error_contains_text!(
        TestValueParserBool::try_parse_from::<&str, &str, &str>(vec![".", "--strict=true"], vec![]),
        [
            "Invalid value",
            "when parsing '--strict' value 'true'",
            "not explicit enough"
        ]
    );

    let result = TestValueParserBool::try_parse_from::<&str, &str, &str>(
        vec![".", "--strict=0", "--flag"],
        vec![],
    )
    .unwrap();

    assert!(result.flag);
    assert!(!result.strict);

    let result = TestValueParserBool::try_parse_from::<&str, &str, &str>(
        vec![".", "--flag"],
        vec![("STRICT", "1")],
    )
    .unwrap();

    assert!(result.flag);
    assert!(result.strict);
}

#[derive(Conf, Debug)]
struct TestValueParserLambda {
    #[conf(short = 't', value_parser = |s: &str| -> Result<_, &'static str> { Ok(s.to_uppercase()) })]
    field: String,
}

#[test]
fn test_value_parser_lambda() {
    assert_error_contains_text!(
        TestValueParserLambda::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["required value was not provided", "'-t'"]
    );

    let result =
        TestValueParserLambda::try_parse_from::<&str, &str, &str>(vec![".", "-t=a"], vec![])
            .unwrap();

    assert_eq!(result.field, "A");
}
