mod common;
use common::*;

use conf::{Conf, ParseType};
use std::borrow::Cow;

/// Test if env aliases work
#[derive(Conf, Debug)]
struct TestEnvAliases {
    /// Required env value with 2 aliases
    #[conf(env, env_aliases = ["REQ", "REQUI"])]
    required: String,
}

#[test]
fn test_env_aliases_get_program_options() {
    let opts = TestEnvAliases::get_program_options().unwrap();

    let mut iter = opts.iter();
    let opt = iter.next().unwrap();

    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form, None);
    assert_eq!(opt.env_form.as_deref(), Some("REQUIRED"));
    assert_eq!(opt.default_value, None);
    assert!(opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Required env value with 2 aliases")
    );
    assert_eq!(
        &opt.env_aliases[..],
        &[Cow::Borrowed("REQ"), Cow::Borrowed("REQUI")]
    );

    assert_eq!(iter.next(), None);
}

#[test]
fn test_env_aliases_parsing() {
    assert_error_contains_text!(
        TestEnvAliases::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["required value was not provided"]
    );

    let result =
        TestEnvAliases::try_parse_from::<&str, &str, &str>(vec!["."], vec![("REQ", "foo")])
            .unwrap();
    assert_eq!(result.required, "foo");

    let result =
        TestEnvAliases::try_parse_from::<&str, &str, &str>(vec!["."], vec![("REQUI", "foo")])
            .unwrap();
    assert_eq!(result.required, "foo");

    let result =
        TestEnvAliases::try_parse_from::<&str, &str, &str>(vec!["."], vec![("REQUIRED", "foo")])
            .unwrap();
    assert_eq!(result.required, "foo");

    let result = TestEnvAliases::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("REQUIRED", "foo"), ("REQUI", "bar")],
    )
    .unwrap();
    assert_eq!(result.required, "foo");

    let result = TestEnvAliases::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("REQ", "baz"), ("REQUI", "bar")],
    )
    .unwrap();
    assert_eq!(result.required, "baz");

    let result = TestEnvAliases::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("REQ", "baz"), ("REQUIRED", "bar")],
    )
    .unwrap();
    assert_eq!(result.required, "bar");
}

/// Test if env aliases work when flattening
#[derive(Conf, Debug)]
struct TestEnvAliases2 {
    #[conf(flatten)]
    a: TestEnvAliases,

    #[conf(flatten, prefix)]
    b: TestEnvAliases,

    #[conf(flatten, env_prefix = "DADA_")]
    c: TestEnvAliases,
}

#[test]
fn test_env_aliases2_get_program_options() {
    let opts = TestEnvAliases2::get_program_options().unwrap();

    let mut iter = opts.iter();
    let opt = iter.next().unwrap();

    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form, None);
    assert_eq!(opt.env_form.as_deref(), Some("REQUIRED"));
    assert_eq!(opt.default_value, None);
    assert!(opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Required env value with 2 aliases")
    );
    assert_eq!(
        &opt.env_aliases[..],
        &[Cow::Borrowed("REQ"), Cow::Borrowed("REQUI")]
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form, None);
    assert_eq!(opt.env_form.as_deref(), Some("B_REQUIRED"));
    assert_eq!(opt.default_value, None);
    assert!(opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Required env value with 2 aliases")
    );
    assert_eq!(
        &opt.env_aliases[..],
        &[Cow::Borrowed("B_REQ"), Cow::Borrowed("B_REQUI")]
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form, None);
    assert_eq!(opt.env_form.as_deref(), Some("DADA_REQUIRED"));
    assert_eq!(opt.default_value, None);
    assert!(opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Required env value with 2 aliases")
    );
    assert_eq!(
        &opt.env_aliases[..],
        &[Cow::Borrowed("DADA_REQ"), Cow::Borrowed("DADA_REQUI")]
    );

    assert_eq!(iter.next(), None);
}

#[test]
fn test_env_aliases2_parsing() {
    assert_error_contains_text!(
        TestEnvAliases2::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        [
            "required value was not provided",
            "'REQUIRED'",
            "'B_REQUIRED'",
            "'DADA_REQUIRED'"
        ]
    );
    assert_error_contains_text!(
        TestEnvAliases2::try_parse_from::<&str, &str, &str>(vec!["."], vec![("REQUIRED", "1")]),
        [
            "required value was not provided",
            "'B_REQUIRED'",
            "'DADA_REQUIRED'"
        ],
        not["'REQUIRED'"]
    );
    assert_error_contains_text!(TestEnvAliases2::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("REQUIRED", "1"), ("B_REQUI", "2")]
    ),
        ["required value was not provided", "'DADA_REQUIRED'"],
        not["'REQUIRED'", "'B_REQUIRED'"]
    );
    let result = TestEnvAliases2::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("REQUIRED", "1"), ("B_REQUI", "2"), ("DADA_REQ", "3")],
    )
    .unwrap();
    assert_eq!(result.a.required, "1");
    assert_eq!(result.b.required, "2");
    assert_eq!(result.c.required, "3");

    let result = TestEnvAliases2::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![
            ("REQUIRED", "1"),
            ("B_REQUI", "2"),
            ("DADA_REQ", "3"),
            ("DADA_REQUI", "4"),
        ],
    )
    .unwrap();
    assert_eq!(result.a.required, "1");
    assert_eq!(result.b.required, "2");
    assert_eq!(result.c.required, "3");

    let result = TestEnvAliases2::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![
            ("REQUIRED", "1"),
            ("B_REQUI", "2"),
            ("DADA_REQ", "3"),
            ("DADA_REQUI", "4"),
            ("B_REQ", "5"),
        ],
    )
    .unwrap();
    assert_eq!(result.a.required, "1");
    assert_eq!(result.b.required, "5");
    assert_eq!(result.c.required, "3");

    let result = TestEnvAliases2::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![
            ("REQUIRED", "1"),
            ("B_REQUI", "2"),
            ("DADA_REQ", "3"),
            ("DADA_REQUI", "4"),
            ("B_REQ", "5"),
            ("REQUI", "6"),
        ],
    )
    .unwrap();
    assert_eq!(result.a.required, "1");
    assert_eq!(result.b.required, "5");
    assert_eq!(result.c.required, "3");

    let result = TestEnvAliases2::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![
            ("REQUIRED", "1"),
            ("B_REQUI", "2"),
            ("DADA_REQ", "3"),
            ("DADA_REQUI", "4"),
            ("B_REQ", "5"),
            ("REQUI", "6"),
            ("DADA_REQUIRED", "7"),
        ],
    )
    .unwrap();
    assert_eq!(result.a.required, "1");
    assert_eq!(result.b.required, "5");
    assert_eq!(result.c.required, "7");
}
