use conf::Conf;

// This should compile but panic when we try to create a parser
#[derive(Conf, Debug)]
#[allow(dead_code)]
struct TestRequiredAfterOptional {
    /// First positional (required)
    #[conf(pos)]
    first: String,

    /// Second positional (optional)
    #[conf(pos)]
    second: Option<String>,

    /// Third positional (optional)
    #[conf(pos)]
    third: Option<String>,

    /// Fourth positional (required) - INVALID!
    #[conf(pos)]
    fourth: String,
}

#[test]
#[should_panic(
    expected = "Required positional argument 'fourth' cannot come after optional positional argument 'third'"
)]
fn test_required_after_optional_panics() {
    // This should panic when we try to create the parser because
    // having a required positional after optional ones is confusing and error-prone
    let _result = TestRequiredAfterOptional::try_parse_from::<&str, &str, &str>(
        vec![".", "arg1", "arg2", "arg3", "arg4"],
        vec![],
    );
}

// Test a valid configuration: all optional positionals at the end
#[derive(Conf, Debug)]
struct TestValidPositionals {
    /// First positional (required)
    #[conf(pos)]
    first: String,

    /// Second positional (required)
    #[conf(pos)]
    second: String,

    /// Third positional (optional) - OK because it's at the end
    #[conf(pos)]
    third: Option<String>,

    /// Fourth positional (optional) - OK because it's at the end
    #[conf(pos)]
    fourth: Option<String>,
}

#[test]
fn test_valid_optional_at_end() {
    // This is valid: required positionals first, then optional ones
    let result =
        TestValidPositionals::try_parse_from::<&str, &str, &str>(vec![".", "arg1", "arg2"], vec![])
            .unwrap();
    assert_eq!(result.first, "arg1");
    assert_eq!(result.second, "arg2");
    assert_eq!(result.third, None);
    assert_eq!(result.fourth, None);

    let result = TestValidPositionals::try_parse_from::<&str, &str, &str>(
        vec![".", "arg1", "arg2", "arg3"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.first, "arg1");
    assert_eq!(result.second, "arg2");
    assert_eq!(result.third, Some("arg3".to_string()));
    assert_eq!(result.fourth, None);

    let result = TestValidPositionals::try_parse_from::<&str, &str, &str>(
        vec![".", "arg1", "arg2", "arg3", "arg4"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.first, "arg1");
    assert_eq!(result.second, "arg2");
    assert_eq!(result.third, Some("arg3".to_string()));
    assert_eq!(result.fourth, Some("arg4".to_string()));
}
