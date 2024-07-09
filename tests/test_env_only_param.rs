mod common;
use common::*;
use conf::{Conf, ParseType};

/// Test what happens when there is a value which can only be read from env.
/// Clap does not officially support this and generates a positional argument,
/// which is usually not desired when using Conf. https://github.com/clap-rs/clap/discussions/5432
#[derive(Conf, Debug)]
struct TestEnvOnly {
    /// Required env value
    #[conf(env)]
    required: String,

    /// Not required env value
    #[conf(env, default_value = "5")]
    not_required: String,
}

#[test]
fn test_env_only_get_program_options() {
    let parser_config = TestEnvOnly::get_parser_config().unwrap();
    let opts = TestEnvOnly::get_program_options().unwrap();

    assert!(!parser_config.no_help_flag);

    let mut iter = opts.iter();

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form, None);
    assert_eq!(opt.env_form.as_deref(), Some("REQUIRED"));
    assert_eq!(opt.default_value, None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Required env value"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form, None);
    assert_eq!(opt.env_form.as_deref(), Some("NOT_REQUIRED"));
    assert_eq!(opt.default_value.as_deref(), Some("5"));
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Not required env value"));

    assert_eq!(iter.next(), None);
}

#[test]
fn test_env_only_parsing() {
    // Missing required env should not work
    assert_error_contains_text!(
        TestEnvOnly::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["required value was not provided", "env 'REQUIRED'"]
    );

    // Supplying them should work
    let result =
        TestEnvOnly::try_parse_from::<&str, &str, &str>(vec!["."], vec![("REQUIRED", "foo")])
            .unwrap();
    assert_eq!(result.required, "foo");
    assert_eq!(result.not_required, "5");

    // Supplying them as a positional argument should not work
    assert_error_contains_text!(
        TestEnvOnly::try_parse_from::<&str, &str, &str>(vec![".", "foo"], vec![]),
        ["unexpected argument 'foo'"]
    );
}
