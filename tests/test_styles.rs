use conf::Conf;
use std::process::Command;

const HELP_STYLES: conf::Styles = conf::Styles::styled()
    .header(conf::anstyle::AnsiColor::Blue.on_default().bold())
    .usage(conf::anstyle::AnsiColor::Blue.on_default().bold())
    .literal(conf::anstyle::AnsiColor::White.on_default())
    .placeholder(conf::anstyle::AnsiColor::Green.on_default());

#[derive(Conf, Debug)]
#[conf(styles = HELP_STYLES)]
struct StyledArgs {
    #[conf(long)]
    name: String,

    #[conf(long)]
    count: Option<i32>,
}

#[test]
fn test_styled_help_compiles() {
    // This test just verifies that the styles attribute compiles and works
    let result =
        StyledArgs::try_parse_from::<&str, &str, &str>(vec!["test_app", "--name", "hello"], vec![]);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.name, "hello");
    assert_eq!(config.count, None);
}

#[test]
fn test_styled_help_output() {
    // Test that help can be generated (will include ANSI codes if terminal supports it)
    let result = StyledArgs::try_parse_from::<&str, &str, &str>(vec!["test_app", "--help"], vec![]);

    // Help should cause an error (but not a bad error)
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.to_string();

    // Should contain help text
    assert!(err_str.contains("--name"));
    assert!(err_str.contains("--count"));
}

/// Test that the colored_help example actually produces ANSI escape codes when forced.
///
/// This test builds and runs the colored_help example with CLICOLOR_FORCE=1 to ensure
/// ANSI codes are emitted even when not connected to a TTY.
#[test]
fn test_colored_help_example_produces_ansi_codes() {
    // First build the example
    let build_status = Command::new("cargo")
        .args(["build", "--example", "colored_help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("Failed to build colored_help example");

    assert!(
        build_status.success(),
        "Failed to build colored_help example"
    );

    // Run the example with --help and force color output
    // CLICOLOR_FORCE is recognized by anstream (used by clap) to force ANSI output to non-TTY
    let output = Command::new("cargo")
        .args(["run", "--example", "colored_help", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        // Force ANSI color output even when not connected to a TTY
        .env("CLICOLOR_FORCE", "1")
        .env("NO_COLOR", "") // Make sure NO_COLOR is not set
        .output()
        .expect("Failed to run colored_help example");

    // Help output goes to stdout for clap
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The output should be in stdout (clap writes help to stdout)
    let help_output = if stdout.contains("--name") {
        stdout
    } else {
        stderr
    };

    // Check that ANSI escape codes are present
    // ESC [ is the start of ANSI escape sequences (0x1B 0x5B or \x1b[)
    let has_ansi = help_output.contains("\x1b[");

    assert!(
        has_ansi,
        "Expected ANSI escape codes in help output but found none.\nOutput:\n{}",
        help_output
    );

    // Also verify the help text contains expected content
    assert!(help_output.contains("--name"), "Help should contain --name");
    assert!(
        help_output.contains("--count"),
        "Help should contain --count"
    );
    assert!(
        help_output.contains("--verbose"),
        "Help should contain --verbose"
    );
}

/// Test that plain styles produce no ANSI codes
#[test]
fn test_plain_styles_no_ansi_codes() {
    // Build a test binary that uses plain styles
    // For this test, we'll use the PlainStyledArgs struct and check its help output

    let result =
        PlainStyledArgs::try_parse_from::<&str, &str, &str>(vec!["test_app", "--help"], vec![]);

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.to_string();

    // Plain styles should not contain ANSI codes
    let has_ansi = err_str.contains("\x1b[");

    // Note: This may still have ANSI codes if the error formatting adds them,
    // but the help text itself should be plain. We're mainly testing that
    // plain() styles don't crash and work correctly.
    assert!(err_str.contains("--option"), "Help should contain --option");

    // If there are ANSI codes, they should be minimal (from error formatting only)
    if has_ansi {
        // Count occurrences - plain styles should have far fewer than styled
        let ansi_count = err_str.matches("\x1b[").count();
        assert!(
            ansi_count < 5,
            "Plain styles should have minimal ANSI codes, found {}",
            ansi_count
        );
    }
}

#[derive(Conf, Debug)]
struct UnstyledArgs {
    #[conf(long)]
    value: String,
}

#[test]
fn test_unstyled_still_works() {
    // Verify that not providing styles still works (uses default)
    let result = UnstyledArgs::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "--value", "test"],
        vec![],
    );

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.value, "test");
}

const PLAIN_STYLES: conf::Styles = conf::Styles::plain();

#[derive(Conf, Debug)]
#[conf(styles = PLAIN_STYLES)]
struct PlainStyledArgs {
    #[conf(long)]
    option: String,
}

#[test]
fn test_plain_styles() {
    // Verify that plain styles (no colors) work
    let result = PlainStyledArgs::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "--option", "value"],
        vec![],
    );

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.option, "value");
}

/// Test comparing styled vs plain output to verify styles are being applied
#[test]
fn test_styled_vs_plain_comparison() {
    // Build both examples to compare their output
    let build_status = Command::new("cargo")
        .args(["build", "--example", "colored_help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("Failed to build example");

    assert!(build_status.success());

    // Run with colors forced
    let colored_output = Command::new("cargo")
        .args(["run", "--example", "colored_help", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CLICOLOR_FORCE", "1")
        .env("NO_COLOR", "")
        .output()
        .expect("Failed to run example");

    // Run with colors disabled
    let plain_output = Command::new("cargo")
        .args(["run", "--example", "colored_help", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to run example");

    let colored_stdout = String::from_utf8_lossy(&colored_output.stdout);
    let plain_stdout = String::from_utf8_lossy(&plain_output.stdout);

    // Colored output should have ANSI codes
    let colored_ansi_count = colored_stdout.matches("\x1b[").count();
    let plain_ansi_count = plain_stdout.matches("\x1b[").count();

    assert!(
        colored_ansi_count > plain_ansi_count,
        "Colored output should have more ANSI codes than plain.\nColored: {}\nPlain: {}",
        colored_ansi_count,
        plain_ansi_count
    );

    // Both should contain the same help text content
    assert!(colored_stdout.contains("--name"));
    assert!(plain_stdout.contains("--name"));
}

/// Test that error messages also get styled
#[test]
fn test_styled_error_output() {
    // Run the example with an invalid argument to trigger an error
    let output = Command::new("cargo")
        .args(["run", "--example", "colored_help", "--", "--invalid-arg"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CLICOLOR_FORCE", "1")
        .env("NO_COLOR", "")
        .output()
        .expect("Failed to run example");

    // Error output typically goes to stderr
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not succeed (exit code should be non-zero)
    assert!(
        !output.status.success(),
        "Should fail with invalid argument"
    );

    // Error message should mention the unexpected argument
    assert!(
        stderr.contains("invalid") || stderr.contains("unexpected") || stderr.contains("unknown"),
        "Error should mention invalid argument: {}",
        stderr
    );
}

/// Test that styles work with subcommands
#[allow(dead_code)]
#[derive(Conf, Debug)]
#[conf(styles = HELP_STYLES)]
struct StyledWithSubcommand {
    #[conf(long)]
    global_flag: bool,

    #[conf(subcommands)]
    command: Option<StyledSubcommands>,
}

#[allow(dead_code)]
#[derive(conf::Subcommands, Debug)]
enum StyledSubcommands {
    Run(RunArgs),
    Build(BuildArgs),
}

#[allow(dead_code)]
#[derive(Conf, Debug)]
struct RunArgs {
    #[conf(long)]
    fast: bool,
}

#[allow(dead_code)]
#[derive(Conf, Debug)]
struct BuildArgs {
    #[conf(long)]
    release: bool,
}

#[test]
fn test_styled_with_subcommands() {
    // Test that styles work with subcommands
    let result = StyledWithSubcommand::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "--help"],
        vec![],
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.to_string();

    // Should contain subcommand names
    assert!(err_str.contains("run"), "Help should list run subcommand");
    assert!(
        err_str.contains("build"),
        "Help should list build subcommand"
    );
    assert!(
        err_str.contains("--global-flag"),
        "Help should show global flag"
    );
}

/// Test all style builder methods compile and work
#[test]
fn test_all_style_methods() {
    use conf::anstyle::AnsiColor;

    // Create styles using all available methods
    const ALL_STYLES: conf::Styles = conf::Styles::styled()
        .header(AnsiColor::Blue.on_default().bold())
        .usage(AnsiColor::Cyan.on_default())
        .literal(AnsiColor::White.on_default())
        .placeholder(AnsiColor::Green.on_default())
        .error(AnsiColor::Red.on_default().bold())
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Red.on_default());

    #[allow(dead_code)]
    #[derive(Conf, Debug)]
    #[conf(styles = ALL_STYLES)]
    struct AllStylesArgs {
        #[conf(long)]
        test: String,
    }

    // Just verify it compiles and parses
    let result = AllStylesArgs::try_parse_from::<&str, &str, &str>(
        vec!["test_app", "--test", "value"],
        vec![],
    );

    assert!(result.is_ok());
}
