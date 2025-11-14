use conf::Conf;

#[derive(Conf, Debug)]
struct TestMultipleOptional {
    /// First positional (required)
    #[conf(pos)]
    first: String,

    /// Second positional (optional)
    #[conf(pos)]
    second: Option<String>,

    /// Third positional (optional)
    #[conf(pos)]
    third: Option<String>,
}

#[test]
fn test_multiple_optional_positionals() {
    // Just the required one
    let result =
        TestMultipleOptional::try_parse_from::<&str, &str, &str>(vec![".", "arg1"], vec![])
            .unwrap();
    assert_eq!(result.first, "arg1");
    assert_eq!(result.second, None);
    assert_eq!(result.third, None);

    // Two arguments - first and second populated
    let result =
        TestMultipleOptional::try_parse_from::<&str, &str, &str>(vec![".", "arg1", "arg2"], vec![])
            .unwrap();
    assert_eq!(result.first, "arg1");
    assert_eq!(result.second, Some("arg2".to_string()));
    assert_eq!(result.third, None);

    // Three arguments - all populated
    let result = TestMultipleOptional::try_parse_from::<&str, &str, &str>(
        vec![".", "arg1", "arg2", "arg3"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.first, "arg1");
    assert_eq!(result.second, Some("arg2".to_string()));
    assert_eq!(result.third, Some("arg3".to_string()));
}

#[test]
fn test_cannot_skip_optional_positional() {
    // This demonstrates that you cannot "skip" the second positional to provide the third
    // If you provide two args, they go to first and second, not first and third
    let result =
        TestMultipleOptional::try_parse_from::<&str, &str, &str>(vec![".", "arg1", "arg2"], vec![])
            .unwrap();

    // second gets the value, not third
    assert_eq!(result.second, Some("arg2".to_string()));
    assert_eq!(result.third, None);
}
