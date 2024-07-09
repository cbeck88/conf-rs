mod common;
use common::*;

use conf::{Conf, ParseType};

#[derive(Conf, Debug)]
struct TestParams {
    /// This is a test param
    #[conf(long, env)]
    required: String,

    // This is an optional param
    #[conf(short, long = "foo", env)]
    my_option: Option<String>,

    /// This is a defaulted param
    #[conf(short, env, default_value = "def")]
    defaulted: String,

    // This is a defaulted optional param
    #[conf(short, env, default_value = "maybe")]
    should_work: Option<String>,
}

#[test]
fn test_params_get_program_options() {
    let parser_config = TestParams::get_parser_config().unwrap();
    let opts = TestParams::get_program_options().unwrap();

    assert!(!parser_config.no_help_flag);
    assert!(parser_config.about.is_none());

    assert_eq!(opts.len(), 4);

    assert_eq!(opts[0].parse_type, ParseType::Parameter);
    assert_eq!(opts[0].short_form, None);
    assert_eq!(opts[0].long_form.as_deref(), Some("required"));
    assert_eq!(opts[0].env_form.as_deref(), Some("REQUIRED"));
    assert_eq!(opts[0].default_value, None);
    assert!(opts[0].is_required);
    assert_eq!(opts[0].description.as_deref(), Some("This is a test param"));

    assert_eq!(opts[1].parse_type, ParseType::Parameter);
    assert_eq!(opts[1].short_form, Some('m'));
    assert_eq!(opts[1].long_form.as_deref(), Some("foo"));
    assert_eq!(opts[1].env_form.as_deref(), Some("MY_OPTION"));
    assert_eq!(opts[1].default_value, None);
    assert!(!opts[1].is_required);
    assert_eq!(opts[1].description.as_deref(), None);

    assert_eq!(opts[2].parse_type, ParseType::Parameter);
    assert_eq!(opts[2].short_form, Some('d'));
    assert_eq!(opts[2].long_form, None);
    assert_eq!(opts[2].env_form.as_deref(), Some("DEFAULTED"));
    assert_eq!(opts[2].default_value.as_deref(), Some("def"));
    assert!(!opts[2].is_required);
    assert_eq!(
        opts[2].description.as_deref(),
        Some("This is a defaulted param")
    );

    assert_eq!(opts[3].parse_type, ParseType::Parameter);
    assert_eq!(opts[3].short_form, Some('s'));
    assert_eq!(opts[3].long_form, None);
    assert_eq!(opts[3].env_form.as_deref(), Some("SHOULD_WORK"));
    assert_eq!(opts[3].default_value.as_deref(), Some("maybe"));
    assert!(!opts[3].is_required);
    assert_eq!(opts[3].description.as_deref(), None);
}

#[test]
fn test_params_parsing() {
    // Missing required params should not work
    assert_error_contains_text!(
        TestParams::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["required value was not provided", "'--required'"]
    );

    // Assigning to a required param should work
    let result =
        TestParams::try_parse_from::<&str, &str, &str>(vec![".", "--required=foo"], vec![])
            .unwrap();
    assert_eq!(result.required, "foo");
    assert_eq!(result.my_option, None);
    assert_eq!(result.defaulted, "def");
    assert_eq!(result.should_work.as_deref(), Some("maybe"));

    // Assigning to a required param using the subsequent cli arg should work
    let result =
        TestParams::try_parse_from::<&str, &str, &str>(vec![".", "--required", "foo"], vec![])
            .unwrap();
    assert_eq!(result.required, "foo");
    assert_eq!(result.my_option, None);
    assert_eq!(result.defaulted, "def");
    assert_eq!(result.should_work.as_deref(), Some("maybe"));

    // Assigning to a required param twice should not work
    assert_error_contains_text!(
        TestParams::try_parse_from::<&str, &str, &str>(
            vec![".", "--required=foo", "--required=foo"],
            vec![]
        ),
        ["cannot be used multiple times"]
    );
    assert_error_contains_text!(
        TestParams::try_parse_from::<&str, &str, &str>(
            vec![".", "--required=foo", "--required", "foo"],
            vec![]
        ),
        ["cannot be used multiple times"]
    );
    assert_error_contains_text!(
        TestParams::try_parse_from::<&str, &str, &str>(
            vec![".", "--required", "foo", "--required", "foo"],
            vec![]
        ),
        ["cannot be used multiple times"]
    );

    // Assigning to a optional param short switch should work similar to a long switch
    assert_error_contains_text!(
        TestParams::try_parse_from::<&str, &str, &str>(vec![".", "--required=foo", "-m"], vec![]),
        ["value is required", "none was supplied"]
    );
    let result = TestParams::try_parse_from::<&str, &str, &str>(
        vec![".", "--required=foo", "-m=59"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.required, "foo");
    assert_eq!(result.my_option.as_deref(), Some("59"));
    assert_eq!(result.defaulted, "def");
    assert_eq!(result.should_work.as_deref(), Some("maybe"));

    // Assigning to optional param short switch using a subsequent cli arg should work
    let result = TestParams::try_parse_from::<&str, &str, &str>(
        vec![".", "--required=foo", "-m", "59"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.required, "foo");
    assert_eq!(result.my_option.as_deref(), Some("59"));
    assert_eq!(result.defaulted, "def");
    assert_eq!(result.should_work.as_deref(), Some("maybe"));
    // Assigning to it twice should not work
    assert_error_contains_text!(
        TestParams::try_parse_from::<&str, &str, &str>(
            vec![".", "--required=foo", "-m=59", "-m=59"],
            vec![]
        ),
        ["cannot be used multiple times"]
    );
    assert_error_contains_text!(
        TestParams::try_parse_from::<&str, &str, &str>(
            vec![".", "--required=foo", "-m=59", "-m", "59"],
            vec![]
        ),
        ["cannot be used multiple times"]
    );
    assert_error_contains_text!(
        TestParams::try_parse_from::<&str, &str, &str>(
            vec![".", "--required=foo", "-m", "59", "-m", "59"],
            vec![]
        ),
        ["cannot be used multiple times"]
    );

    // Setting the optional defaulted parameter should work
    let result = TestParams::try_parse_from::<&str, &str, &str>(
        vec![".", "--required=foo", "-m=59", "-s", "yes"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.required, "foo");
    assert_eq!(result.my_option.as_deref(), Some("59"));
    assert_eq!(result.defaulted, "def");
    assert_eq!(result.should_work.as_deref(), Some("yes"));

    // Setting the defaulted parameter via env should work
    let result = TestParams::try_parse_from::<&str, &str, &str>(
        vec![".", "--required=foo", "-m=59", "-s", "yes"],
        vec![("DEFAULTED", "deff")],
    )
    .unwrap();
    assert_eq!(result.required, "foo");
    assert_eq!(result.my_option.as_deref(), Some("59"));
    assert_eq!(result.defaulted, "deff");
    assert_eq!(result.should_work.as_deref(), Some("yes"));

    // Setting the defaulted parameter via switch and env should work, and switch should shadow env
    let result = TestParams::try_parse_from::<&str, &str, &str>(
        vec![".", "--required=foo", "-m=59", "-s", "yes", "-d=bueno"],
        vec![("DEFAULTED", "deff")],
    )
    .unwrap();
    assert_eq!(result.required, "foo");
    assert_eq!(result.my_option.as_deref(), Some("59"));
    assert_eq!(result.defaulted, "bueno");
    assert_eq!(result.should_work.as_deref(), Some("yes"));

    // Using = in a cli value should work
    let result = TestParams::try_parse_from::<&str, &str, &str>(
        vec![".", "--required=foo=bar", "-m=59", "-s", "yes", "-d=bueno"],
        vec![("DEFAULTED", "deff")],
    )
    .unwrap();
    assert_eq!(result.required, "foo=bar");
    assert_eq!(result.my_option.as_deref(), Some("59"));
    assert_eq!(result.defaulted, "bueno");
    assert_eq!(result.should_work.as_deref(), Some("yes"));

    let result = TestParams::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--required=foo=bar",
            "-m=59",
            "-s",
            "yes=no",
            "-d=bueno",
        ],
        vec![("DEFAULTED", "deff")],
    )
    .unwrap();
    assert_eq!(result.required, "foo=bar");
    assert_eq!(result.my_option.as_deref(), Some("59"));
    assert_eq!(result.defaulted, "bueno");
    assert_eq!(result.should_work.as_deref(), Some("yes=no"));

    let result = TestParams::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--required=foo=bar",
            "-m==59",
            "-s",
            "yes=no",
            "-d=bueno",
        ],
        vec![("DEFAULTED", "deff")],
    )
    .unwrap();
    assert_eq!(result.required, "foo=bar");
    assert_eq!(result.my_option.as_deref(), Some("=59"));
    assert_eq!(result.defaulted, "bueno");
    assert_eq!(result.should_work.as_deref(), Some("yes=no"));

    // Using -- in a cli value should not work (unless allow_hyphen_values is on)
    assert_error_contains_text!(
        TestParams::try_parse_from::<&str, &str, &str>(
            vec![
                ".",
                "--required=--foo=bar",
                "-m=59",
                "-s",
                "--yes=no",
                "-d=bueno",
            ],
            vec![("DEFAULTED", "deff")],
        ),
        ["unexpected argument '--yes'"]
    );

    // Using = should work even if the value has hyphens
    let result = TestParams::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--required=foo=bar",
            "-m==59",
            "-s=--yes=no",
            "-d=bueno",
        ],
        vec![("DEFAULTED", "deff")],
    )
    .unwrap();
    assert_eq!(result.required, "foo=bar");
    assert_eq!(result.my_option.as_deref(), Some("=59"));
    assert_eq!(result.defaulted, "bueno");
    assert_eq!(result.should_work.as_deref(), Some("--yes=no"));
}
