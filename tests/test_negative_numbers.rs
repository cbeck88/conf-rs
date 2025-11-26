mod common;
use common::*;

use conf::Conf;

#[derive(Conf, Debug)]
struct SignedTypes {
    /// Integer parameter
    #[conf(long)]
    int_value: i32,

    /// Float parameter
    #[conf(long)]
    float_value: f64,

    /// Repeat integers
    #[conf(repeat, long)]
    int_list: Vec<i32>,

    /// Repeat floats
    #[conf(repeat, long)]
    float_list: Vec<f64>,
}

#[derive(Conf, Debug)]
struct StringTypesWithoutAllowNegative {
    /// String parameter without allow_negative_numbers
    #[conf(long)]
    value: String,

    /// Repeat strings without allow_negative_numbers
    #[conf(repeat, long)]
    values: Vec<String>,
}

#[derive(Conf, Debug)]
struct ExplicitAllowNegativeNumbers {
    /// String parameter with explicit allow_negative_numbers
    #[conf(long, allow_negative_numbers)]
    value: String,

    /// Repeat strings with explicit allow_negative_numbers
    #[conf(repeat, long, allow_negative_numbers)]
    values: Vec<String>,
}

#[derive(Conf, Debug)]
struct ExplicitAllowHyphenValues {
    /// String parameter with explicit allow_hyphen_values
    #[conf(long, allow_hyphen_values)]
    value: String,

    /// Repeat strings with explicit allow_hyphen_values
    #[conf(repeat, long, allow_hyphen_values)]
    values: Vec<String>,
}

#[test]
fn test_signed_integer_accepts_negative_values() {
    // Test parameter with negative value
    let result = SignedTypes::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--int-value",
            "-2",
            "--float-value",
            "3.14",
            "--int-list",
            "1",
            "--float-list",
            "2.0",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.int_value, -2);
    assert_eq!(result.float_value, 3.14);

    // Test parameter with negative value using = syntax
    let result = SignedTypes::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--int-value=-2",
            "--float-value=3.14",
            "--int-list=1",
            "--float-list=2.0",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.int_value, -2);
    assert_eq!(result.float_value, 3.14);
}

#[test]
fn test_signed_float_accepts_negative_values() {
    // Test parameter with negative value
    let result = SignedTypes::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--int-value",
            "5",
            "--float-value",
            "-2.5",
            "--int-list",
            "1",
            "--float-list",
            "2.0",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.int_value, 5);
    assert_eq!(result.float_value, -2.5);

    // Test parameter with negative value using = syntax
    let result = SignedTypes::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--int-value=5",
            "--float-value=-2.5",
            "--int-list=1",
            "--float-list=2.0",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.int_value, 5);
    assert_eq!(result.float_value, -2.5);
}

#[test]
fn test_repeat_signed_integers_accept_negative_values() {
    let result = SignedTypes::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--int-value=0",
            "--float-value=0.0",
            "--int-list",
            "-2",
            "--int-list",
            "5",
            "--int-list",
            "-10",
            "--float-list=1.0",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.int_list, vec![-2, 5, -10]);

    // Test with = syntax
    let result = SignedTypes::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--int-value=0",
            "--float-value=0.0",
            "--int-list=-2",
            "--int-list=5",
            "--int-list=-10",
            "--float-list=1.0",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.int_list, vec![-2, 5, -10]);
}

#[test]
fn test_repeat_signed_floats_accept_negative_values() {
    let result = SignedTypes::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--int-value=0",
            "--float-value=0.0",
            "--int-list=1",
            "--float-list",
            "-2.5",
            "--float-list",
            "3.14",
            "--float-list",
            "-1.0",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.float_list, vec![-2.5, 3.14, -1.0]);

    // Test with = syntax
    let result = SignedTypes::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--int-value=0",
            "--float-value=0.0",
            "--int-list=1",
            "--float-list=-2.5",
            "--float-list=3.14",
            "--float-list=-1.0",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.float_list, vec![-2.5, 3.14, -1.0]);
}

#[test]
fn test_signed_types_still_reject_non_numeric_hyphen_values() {
    // Should still reject things that look like flags, not negative numbers
    assert_error_contains_text!(
        SignedTypes::try_parse_from::<&str, &str, &str>(
            vec![
                ".",
                "--int-value",
                "--not-a-number",
                "--float-value=0.0",
                "--int-list=1",
                "--float-list=1.0",
            ],
            vec![],
        ),
        ["unexpected argument '--not-a-number'"]
    );
}

#[test]
fn test_string_types_reject_negative_number_syntax_without_equals() {
    // String types don't have allow_negative_numbers automatically enabled
    // So using space syntax with -2 should fail (it looks like an unexpected argument)
    assert_error_contains_text!(
        StringTypesWithoutAllowNegative::try_parse_from::<&str, &str, &str>(
            vec![".", "--value", "-2", "--values=foo"],
            vec![],
        ),
        ["unexpected argument '-2'"]
    );

    // But using = syntax should still work
    let result = StringTypesWithoutAllowNegative::try_parse_from::<&str, &str, &str>(
        vec![".", "--value=-2", "--values=foo"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.value, "-2");

    // Same for repeat fields
    assert_error_contains_text!(
        StringTypesWithoutAllowNegative::try_parse_from::<&str, &str, &str>(
            vec![".", "--value=test", "--values", "-2"],
            vec![],
        ),
        ["unexpected argument '-2'"]
    );

    // But = syntax works
    let result = StringTypesWithoutAllowNegative::try_parse_from::<&str, &str, &str>(
        vec![".", "--value=test", "--values=-2"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.values, vec!["-2"]);
}

#[test]
fn test_explicit_allow_negative_numbers_on_string() {
    // With explicit allow_negative_numbers, String fields should accept negative number syntax
    let result = ExplicitAllowNegativeNumbers::try_parse_from::<&str, &str, &str>(
        vec![".", "--value", "-42", "--values=1"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.value, "-42");

    // Should work with = syntax too
    let result = ExplicitAllowNegativeNumbers::try_parse_from::<&str, &str, &str>(
        vec![".", "--value=-42", "--values=1"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.value, "-42");

    // Should work with floats too
    let result = ExplicitAllowNegativeNumbers::try_parse_from::<&str, &str, &str>(
        vec![".", "--value", "-3.14", "--values=1"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.value, "-3.14");

    // But should still reject things that don't look like numbers
    assert_error_contains_text!(
        ExplicitAllowNegativeNumbers::try_parse_from::<&str, &str, &str>(
            vec![".", "--value", "--not-a-number", "--values=1"],
            vec![],
        ),
        ["unexpected argument '--not-a-number'"]
    );
}

#[test]
fn test_explicit_allow_negative_numbers_on_repeat() {
    let result = ExplicitAllowNegativeNumbers::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--value=test",
            "--values",
            "-2",
            "--values",
            "5",
            "--values",
            "-10",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.values, vec!["-2", "5", "-10"]);
}

#[test]
fn test_explicit_allow_hyphen_values_on_string() {
    // With explicit allow_hyphen_values, String fields should accept any hyphenated value
    let result = ExplicitAllowHyphenValues::try_parse_from::<&str, &str, &str>(
        vec![".", "--value", "--some-flag", "--values=1"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.value, "--some-flag");

    // Should work with = syntax too
    let result = ExplicitAllowHyphenValues::try_parse_from::<&str, &str, &str>(
        vec![".", "--value=--some-flag", "--values=1"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.value, "--some-flag");

    // Should work with arbitrary hyphenated strings
    let result = ExplicitAllowHyphenValues::try_parse_from::<&str, &str, &str>(
        vec![".", "--value", "-foo-bar-baz", "--values=1"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.value, "-foo-bar-baz");
}

#[test]
fn test_explicit_allow_hyphen_values_on_repeat() {
    let result = ExplicitAllowHyphenValues::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--value=test",
            "--values",
            "--foo",
            "--values",
            "-bar",
            "--values",
            "normal",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.values, vec!["--foo", "-bar", "normal"]);
}
