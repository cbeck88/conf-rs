mod common;
use common::*;

use conf::Conf;

#[derive(Conf, Debug)]
struct TestDefaultIfMissing {
    /// Port number with default_if_missing
    #[conf(long, default_if_missing = "8080")]
    port: Option<u16>,

    /// Host with default_if_missing and default_value
    #[conf(long, default_value = "localhost", default_if_missing = "0.0.0.0")]
    host: String,

    /// Required parameter that has default_if_missing
    #[conf(short, long, default_if_missing = "fallback")]
    name: String,

    /// Optional parameter with default_if_missing
    #[conf(short, long, default_if_missing = "default")]
    config: Option<String>,
}

#[test]
fn test_default_if_missing_program_options() {
    let opts = TestDefaultIfMissing::PROGRAM_OPTIONS
        .iter()
        .collect::<Vec<_>>();

    assert_eq!(opts.len(), 4);

    // Check that default_if_missing is set correctly
    assert_eq!(opts[0].id, "port");
    assert_eq!(opts[0].default_if_missing.as_deref(), Some("8080"));

    assert_eq!(opts[1].id, "host");
    assert_eq!(opts[1].default_if_missing.as_deref(), Some("0.0.0.0"));

    assert_eq!(opts[2].id, "name");
    assert_eq!(opts[2].default_if_missing.as_deref(), Some("fallback"));

    assert_eq!(opts[3].id, "config");
    assert_eq!(opts[3].default_if_missing.as_deref(), Some("default"));
}

#[test]
fn test_default_if_missing_when_absent() {
    // When the parameter is completely absent, it should use the normal default behavior
    // port is optional, so it should be None
    // host has default_value, so it should use that
    // name is required, so it should error
    assert_error_contains_text!(
        TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["required value was not provided", "'--name'"]
    );

    // With name provided, port should be None and host should be "localhost"
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "--name=test"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.port, None);
    assert_eq!(result.host, "localhost");
    assert_eq!(result.name, "test");
    assert_eq!(result.config, None);
}

#[test]
fn test_default_if_missing_with_value() {
    // When the parameter appears with a value, that value should be used
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "--name=test", "--port=3000", "--host=example.com"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.port, Some(3000));
    assert_eq!(result.host, "example.com");
    assert_eq!(result.name, "test");
    assert_eq!(result.config, None);

    // Test using space-separated syntax
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "--name", "test", "--port", "3000", "--host", "example.com"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.port, Some(3000));
    assert_eq!(result.host, "example.com");
    assert_eq!(result.name, "test");
    assert_eq!(result.config, None);
}

#[test]
fn test_default_if_missing_without_value() {
    // When the parameter appears without a value, default_if_missing should be used
    // This is the key feature: --port (without a value) should use "8080"
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "--name=test", "--port"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.port, Some(8080));
    assert_eq!(result.host, "localhost");
    assert_eq!(result.name, "test");
    assert_eq!(result.config, None);

    // Test with host as well
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "--name=test", "--port", "--host"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.port, Some(8080));
    assert_eq!(result.host, "0.0.0.0");
    assert_eq!(result.name, "test");
    assert_eq!(result.config, None);

    // Test with required parameter using default_if_missing
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "--name"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.port, None);
    assert_eq!(result.host, "localhost");
    assert_eq!(result.name, "fallback");
    assert_eq!(result.config, None);

    // Test with short form
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "-n", "-c"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.port, None);
    assert_eq!(result.host, "localhost");
    assert_eq!(result.name, "fallback");
    assert_eq!(result.config, Some("default".to_string()));
}

#[test]
fn test_default_if_missing_mixed_scenarios() {
    // Mix of present with value, present without value, and absent
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "--name=myname", "--port", "--host=custom.com"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.port, Some(8080)); // present without value -> default_if_missing
    assert_eq!(result.host, "custom.com"); // present with value
    assert_eq!(result.name, "myname"); // present with value
    assert_eq!(result.config, None); // absent -> None

    // All using default_if_missing
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "--name", "--port", "--host"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.port, Some(8080));
    assert_eq!(result.host, "0.0.0.0");
    assert_eq!(result.name, "fallback");
    assert_eq!(result.config, None);

    // All with values
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--name=custom",
            "--port=9000",
            "--host=localhost",
            "--config=prod.conf",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.port, Some(9000));
    assert_eq!(result.host, "localhost");
    assert_eq!(result.name, "custom");
    assert_eq!(result.config, Some("prod.conf".to_string()));
}

#[test]
fn test_default_if_missing_with_equals_syntax() {
    // When using --flag= (equals but no value), it should be treated as empty string, not missing
    // This is standard clap behavior
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "--name=", "--config=test"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.name, ""); // Empty string, not "fallback"
    assert_eq!(result.config, Some("test".to_string()));

    // But --flag (without equals) should use default_if_missing
    let result = TestDefaultIfMissing::try_parse_from::<&str, &str, &str>(
        vec![".", "--name", "--config=test"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.name, "fallback"); // Uses default_if_missing
    assert_eq!(result.config, Some("test".to_string()));
}
