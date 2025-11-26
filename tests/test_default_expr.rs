use conf::Conf;

/// Test default attribute with no parens (uses Default::default())
#[test]
fn test_default_no_parens() {
    #[derive(Conf, Debug)]
    struct Config {
        #[conf(long, default, default_help_str = "0")]
        value: i32,
    }

    // Without providing the value, should use Default::default()
    let result = Config::try_parse_from::<&str, &str, &str>(vec!["test_app"], vec![]).unwrap();
    assert_eq!(result.value, 0);

    // With value provided
    let result =
        Config::try_parse_from::<&str, &str, &str>(vec!["test_app", "--value", "42"], vec![])
            .unwrap();
    assert_eq!(result.value, 42);
}

/// Test default attribute with literal expression
#[test]
fn test_default_with_literal() {
    #[derive(Conf, Debug)]
    struct Config {
        #[conf(long, default(5))]
        value: i32,
    }

    // Without providing the value, should use 5
    let result = Config::try_parse_from::<&str, &str, &str>(vec!["test_app"], vec![]).unwrap();
    assert_eq!(result.value, 5);

    // With value provided
    let result =
        Config::try_parse_from::<&str, &str, &str>(vec!["test_app", "--value", "42"], vec![])
            .unwrap();
    assert_eq!(result.value, 42);
}

/// Test default attribute with expression
#[test]
fn test_default_with_expression() {
    #[derive(Conf, Debug)]
    struct Config {
        #[conf(long, default(10 + 5), default_help_str = "15")]
        value: i32,
    }

    // Without providing the value, should use 15
    let result = Config::try_parse_from::<&str, &str, &str>(vec!["test_app"], vec![]).unwrap();
    assert_eq!(result.value, 15);

    // With value provided
    let result =
        Config::try_parse_from::<&str, &str, &str>(vec!["test_app", "--value", "42"], vec![])
            .unwrap();
    assert_eq!(result.value, 42);
}

/// Test that default bypasses value_parser
#[test]
fn test_default_bypasses_value_parser() {
    #[derive(Debug, PartialEq)]
    struct MyType(String);

    #[derive(Conf, Debug)]
    struct Config {
        #[conf(
            long,
            default(MyType("from_default".to_string())),
            default_help_str = "MyType(\"from_default\")",
            value_parser = |s: &str| Ok::<_, String>(MyType(format!("parsed_{}", s)))
        )]
        my_value: MyType,
    }

    // Without providing the value, should use default expression (bypass value_parser)
    let result = Config::try_parse_from::<&str, &str, &str>(vec!["test_app"], vec![]).unwrap();
    assert_eq!(result.my_value, MyType("from_default".to_string()));

    // With value provided, should go through value_parser
    let result =
        Config::try_parse_from::<&str, &str, &str>(vec!["test_app", "--my-value", "foo"], vec![])
            .unwrap();
    assert_eq!(result.my_value, MyType("parsed_foo".to_string()));
}

/// Test default with Option<T>
#[test]
fn test_default_with_option() {
    #[derive(Conf, Debug)]
    struct Config {
        #[conf(long, default(42), default_help_str = "42")]
        value: Option<i32>,
    }

    // Without providing the value, should use Some(42)
    let result = Config::try_parse_from::<&str, &str, &str>(vec!["test_app"], vec![]).unwrap();
    assert_eq!(result.value, Some(42));

    // With value provided
    let result =
        Config::try_parse_from::<&str, &str, &str>(vec!["test_app", "--value", "99"], vec![])
            .unwrap();
    assert_eq!(result.value, Some(99));
}

/// Test default with env
#[test]
fn test_default_with_env() {
    #[derive(Conf, Debug)]
    struct Config {
        #[conf(long, env, default(10))]
        value: i32,
    }

    // Without args or env, should use default
    let result = Config::try_parse_from::<&str, &str, &str>(vec!["test_app"], vec![]).unwrap();
    assert_eq!(result.value, 10);

    // With env provided
    let result =
        Config::try_parse_from::<&str, &str, &str>(vec!["test_app"], vec![("VALUE", "42")])
            .unwrap();
    assert_eq!(result.value, 42);

    // Args take precedence over default
    let result =
        Config::try_parse_from::<&str, &str, &str>(vec!["test_app", "--value", "99"], vec![])
            .unwrap();
    assert_eq!(result.value, 99);
}

/// Test that help displays the stringified literal
#[test]
fn test_help_with_literal_default() {
    #[derive(Conf, Debug)]
    #[allow(dead_code)]
    struct Config {
        /// My value field
        #[conf(long, default(42))]
        value: i32,
    }

    let env = Default::default();
    let opts = Config::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();
    let parser = Config::get_parser(&env, opts).unwrap();
    let help = parser.render_clap_help();

    // Should show the default value
    assert!(help.contains("42"));
    assert!(help.contains("My value field"));
}
