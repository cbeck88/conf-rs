use conf::Conf;

/// Test with #[conf(version)] - uses CARGO_PKG_VERSION
#[derive(Conf, Debug)]
#[conf(version)]
#[allow(dead_code)]
struct ConfigWithVersion {
    #[conf(long, env)]
    some_value: Option<String>,
}

/// Test with #[conf(version = "1.2.3")] - explicit version
#[derive(Conf, Debug)]
#[conf(version = "1.2.3")]
#[allow(dead_code)]
struct ConfigWithExplicitVersion {
    #[conf(long, env)]
    some_value: Option<String>,
}

/// Test with #[conf(version_fn = my_version_fn)] - function returning version
fn my_version_fn() -> &'static str {
    "custom-version-from-fn"
}

#[derive(Conf, Debug)]
#[conf(version_fn = my_version_fn)]
#[allow(dead_code)]
struct ConfigWithVersionFn {
    #[conf(long, env)]
    some_value: Option<String>,
}

/// Test that --version causes early exit even with required args missing
#[derive(Conf, Debug)]
#[conf(version)]
#[allow(dead_code)]
struct ConfigWithRequiredArg {
    #[conf(long, env)]
    required_value: String,
}

#[test]
fn test_version_flag_early_exit() {
    // --version should cause early exit (exit code 0), not error about missing required arg
    let args = vec!["test", "--version"];
    let env: Vec<(String, String)> = vec![];

    let result = ConfigWithRequiredArg::try_parse_from::<_, _, String>(args, env);
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Version display has exit code 0
    assert_eq!(err.exit_code(), 0);
}

#[test]
fn test_version_flag_short() {
    // -V should also work
    let args = vec!["test", "-V"];
    let env: Vec<(String, String)> = vec![];

    let result = ConfigWithRequiredArg::try_parse_from::<_, _, String>(args, env);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.exit_code(), 0);
}

#[test]
fn test_explicit_version_string() {
    let args = vec!["test", "--version"];
    let env: Vec<(String, String)> = vec![];

    let result = ConfigWithExplicitVersion::try_parse_from::<_, _, String>(args, env);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.exit_code(), 0);
    // The version output should contain "1.2.3"
    let output = format!("{}", err);
    assert!(
        output.contains("1.2.3"),
        "Expected version output to contain '1.2.3', got: {}",
        output
    );
}

#[test]
fn test_version_fn() {
    let args = vec!["test", "--version"];
    let env: Vec<(String, String)> = vec![];

    let result = ConfigWithVersionFn::try_parse_from::<_, _, String>(args, env);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.exit_code(), 0);
    // The version output should contain the custom version from the function
    let output = format!("{}", err);
    assert!(
        output.contains("custom-version-from-fn"),
        "Expected version output to contain 'custom-version-from-fn', got: {}",
        output
    );
}

#[test]
fn test_no_version_flag_without_attribute() {
    // Config without version attribute should not have --version flag
    #[derive(Conf, Debug)]
    #[allow(dead_code)]
    struct ConfigNoVersion {
        #[conf(long, env)]
        some_value: Option<String>,
    }

    let args = vec!["test", "--version"];
    let env: Vec<(String, String)> = vec![];

    let result = ConfigNoVersion::try_parse_from::<_, _, String>(args, env);
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Should be an actual error (unknown flag), not version display (exit code != 0)
    assert_ne!(
        err.exit_code(),
        0,
        "Expected error exit code, but got 0 (version display)"
    );
}

#[test]
fn test_normal_parsing_still_works_with_version() {
    let args = vec!["test", "--some-value", "hello"];
    let env: Vec<(String, String)> = vec![];

    let config = ConfigWithVersion::try_parse_from::<_, _, String>(args, env).unwrap();
    assert_eq!(config.some_value, Some("hello".to_string()));
}
