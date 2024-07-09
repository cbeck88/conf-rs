mod common;
use common::*;
use conf::{Conf, ParseType};

#[derive(Conf, Debug, Eq, PartialEq)]
struct ExampleStruct {
    #[conf(long, env)]
    required: String,

    #[conf(long)]
    also: String,

    #[conf(long, env)]
    opt: Option<String>,

    #[conf(short, long)]
    flag: bool,
}

#[derive(Conf, Debug)]
struct TestFlattenOptional {
    #[conf(flatten)]
    a: Option<ExampleStruct>,

    #[conf(flatten, prefix, skip_short=['f'])]
    b: Option<ExampleStruct>,
}

#[test]
fn test_flatten_optional_get_program_options() {
    let opts = TestFlattenOptional::get_program_options().unwrap();

    let mut iter = opts.iter();

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("required"));
    assert_eq!(opt.env_form.as_deref(), Some("REQUIRED"));
    assert_eq!(opt.default_value, None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), None);

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("also"));
    assert_eq!(opt.env_form.as_deref(), None);
    assert_eq!(opt.default_value, None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), None);

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("opt"));
    assert_eq!(opt.env_form.as_deref(), Some("OPT"));
    assert_eq!(opt.default_value, None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), None);

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Flag);
    assert_eq!(opt.short_form, Some('f'));
    assert_eq!(opt.long_form.as_deref(), Some("flag"));
    assert_eq!(opt.env_form.as_deref(), None);
    assert_eq!(opt.default_value, None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), None);

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("b-required"));
    assert_eq!(opt.env_form.as_deref(), Some("B_REQUIRED"));
    assert_eq!(opt.default_value, None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), None);

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("b-also"));
    assert_eq!(opt.env_form.as_deref(), None);
    assert_eq!(opt.default_value, None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), None);

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("b-opt"));
    assert_eq!(opt.env_form.as_deref(), Some("B_OPT"));
    assert_eq!(opt.default_value, None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), None);

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Flag);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("b-flag"));
    assert_eq!(opt.env_form.as_deref(), None);
    assert_eq!(opt.default_value, None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), None);

    assert_eq!(iter.next(), None);
}

#[test]
fn test_flatten_optional_parsing() {
    let result =
        TestFlattenOptional::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert_eq!(result.a, None);
    assert_eq!(result.b, None);

    assert_error_contains_text!(
        TestFlattenOptional::try_parse_from::<&str, &str, &str>(vec!["."], vec![("REQUIRED", "1")]),
        [
            "required value was not provided",
            "'--also' must be provided",
            "because env 'REQUIRED' was provided"
        ],
        not["--required"]
    );
    assert_error_contains_text!(
        TestFlattenOptional::try_parse_from::<&str, &str, &str>(vec![".", "--also=2"], vec![]),
        [
            "required value was not provided",
            "'--required', must be provided",
            "because '--also' was provided"
        ],
        not["--also' must be provided"]
    );

    let result = TestFlattenOptional::try_parse_from::<&str, &str, &str>(
        vec![".", "--also=2"],
        vec![("REQUIRED", "1")],
    )
    .unwrap();
    let a = result.a.as_ref().unwrap();
    assert_eq!(a.required, "1");
    assert_eq!(a.also, "2");
    assert_eq!(a.opt, None);
    assert_eq!(a.flag, false);
    assert_eq!(result.b, None);

    let result = TestFlattenOptional::try_parse_from::<&str, &str, &str>(
        vec![".", "--also=2", "--flag"],
        vec![("REQUIRED", "1")],
    )
    .unwrap();
    let a = result.a.as_ref().unwrap();
    assert_eq!(a.required, "1");
    assert_eq!(a.also, "2");
    assert_eq!(a.opt, None);
    assert_eq!(a.flag, true);
    assert_eq!(result.b, None);

    assert_error_contains_text!(
        TestFlattenOptional::try_parse_from::<&str, &str, &str>(
            vec![".", "--flag"],
            vec![("REQUIRED", "1")]
        ),
        [
            "required value was not provided",
            "'--also' must be provided",
            "because env 'REQUIRED'"
        ],
        not["--required"]
    );
    assert_error_contains_text!(
        TestFlattenOptional::try_parse_from::<&str, &str, &str>(
            vec![".", "--flag"],
            vec![("REQUIRED", "1"), ("OPT", "3")]
        ),
        [
            "required value was not provided",
            "'--also' must be provided",
            "because env 'REQUIRED'"
        ],
        not["--required"]
    );

    let result = TestFlattenOptional::try_parse_from::<&str, &str, &str>(
        vec![".", "--also", "4", "-f"],
        vec![("REQUIRED", "1"), ("OPT", "3")],
    )
    .unwrap();
    let a = result.a.as_ref().unwrap();
    assert_eq!(a.required, "1");
    assert_eq!(a.also, "4");
    assert_eq!(a.opt.as_deref(), Some("3"));
    assert_eq!(a.flag, true);
    assert_eq!(result.b, None);

    assert_error_contains_text!(
        TestFlattenOptional::try_parse_from::<&str, &str, &str>(
            vec![".", "--also", "4", "-f"],
            vec![("REQUIRED", "1"), ("OPT", "3"), ("B_OPT", "5")]
        ),
        [
            "required value was not provided",
            "'--b-required', must be provided",
            "because env 'B_OPT'"
        ]
    );
    assert_error_contains_text!(
        TestFlattenOptional::try_parse_from::<&str, &str, &str>(
            vec![".", "--also", "4", "-f", "--b-flag"],
            vec![("REQUIRED", "1"), ("OPT", "3"), ("B_OPT", "5")]
        ),
        [
            "required value was not provided",
            "'--b-required', must be provided",
            "because env 'B_OPT'"
        ]
    );
    assert_error_contains_text!(
        TestFlattenOptional::try_parse_from::<&str, &str, &str>(
            vec![".", "--also", "4", "-f", "--b-flag", "--b-also=6"],
            vec![("REQUIRED", "1"), ("OPT", "3"), ("B_OPT", "5")]
        ),
        [
            "required value was not provided",
            "'--b-required', must be provided",
            "because '--b-also'"
        ]
    );

    let result = TestFlattenOptional::try_parse_from::<&str, &str, &str>(
        vec![".", "--also", "4", "-f", "--b-flag", "--b-also=6"],
        vec![
            ("REQUIRED", "1"),
            ("OPT", "3"),
            ("B_OPT", "5"),
            ("B_REQUIRED", "7"),
        ],
    )
    .unwrap();
    let a = result.a.as_ref().unwrap();
    assert_eq!(a.required, "1");
    assert_eq!(a.also, "4");
    assert_eq!(a.opt.as_deref(), Some("3"));
    assert_eq!(a.flag, true);
    let b = result.b.as_ref().unwrap();
    assert_eq!(b.required, "7");
    assert_eq!(b.also, "6");
    assert_eq!(b.opt.as_deref(), Some("5"));
    assert_eq!(b.flag, true);
}
