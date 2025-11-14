mod common;
use common::*;

use conf::{Conf, ParseType};

#[derive(Conf, Debug)]
struct TestPositional {
    /// Input file path
    #[conf(pos)]
    input: String,

    /// Output file path (optional)
    #[conf(pos)]
    output: Option<String>,

    /// Verbose flag
    #[conf(short, long)]
    verbose: bool,
}

#[test]
fn test_positional_get_program_options() {
    let opts = TestPositional::get_program_options().unwrap();

    assert_eq!(opts.len(), 3);

    // First positional argument
    assert_eq!(opts[0].parse_type, ParseType::Parameter);
    assert_eq!(opts[0].short_form, None);
    assert_eq!(opts[0].long_form, None);
    assert_eq!(opts[0].env_form, None);
    assert!(opts[0].is_positional);
    assert!(opts[0].is_required);
    assert_eq!(opts[0].description.as_deref(), Some("Input file path"));

    // Second positional argument (optional)
    assert_eq!(opts[1].parse_type, ParseType::Parameter);
    assert_eq!(opts[1].short_form, None);
    assert_eq!(opts[1].long_form, None);
    assert_eq!(opts[1].env_form, None);
    assert!(opts[1].is_positional);
    assert!(!opts[1].is_required);
    assert_eq!(
        opts[1].description.as_deref(),
        Some("Output file path (optional)")
    );

    // Verbose flag
    assert_eq!(opts[2].parse_type, ParseType::Flag);
    assert!(!opts[2].is_positional);
}

#[test]
fn test_positional_parsing() {
    // Missing required positional should fail
    assert_error_contains_text!(
        TestPositional::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["<input>"]
    );

    // Providing just the required positional should work
    let result =
        TestPositional::try_parse_from::<&str, &str, &str>(vec![".", "input.txt"], vec![]).unwrap();
    assert_eq!(result.input, "input.txt");
    assert_eq!(result.output, None);
    assert!(!result.verbose);

    // Providing both positionals should work
    let result = TestPositional::try_parse_from::<&str, &str, &str>(
        vec![".", "input.txt", "output.txt"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.input, "input.txt");
    assert_eq!(result.output, Some("output.txt".to_string()));
    assert!(!result.verbose);

    // Positionals with flags
    let result = TestPositional::try_parse_from::<&str, &str, &str>(
        vec![".", "input.txt", "--verbose"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.input, "input.txt");
    assert_eq!(result.output, None);
    assert!(result.verbose);

    // Positionals with flags interleaved
    let result = TestPositional::try_parse_from::<&str, &str, &str>(
        vec![".", "--verbose", "input.txt", "output.txt"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.input, "input.txt");
    assert_eq!(result.output, Some("output.txt".to_string()));
    assert!(result.verbose);
}

#[test]
fn test_positional_with_env() {
    #[derive(Conf, Debug)]
    struct TestPosWithEnv {
        /// Input from CLI or env
        #[conf(pos, env)]
        input: String,
    }

    let opts = TestPosWithEnv::get_program_options().unwrap();
    assert_eq!(opts.len(), 1);
    assert!(opts[0].is_positional);
    assert_eq!(opts[0].env_form.as_deref(), Some("INPUT"));

    // Can be provided via positional
    let result =
        TestPosWithEnv::try_parse_from::<&str, &str, &str>(vec![".", "from_args"], vec![]).unwrap();
    assert_eq!(result.input, "from_args");

    // Can be provided via env
    let result =
        TestPosWithEnv::try_parse_from::<&str, &str, &str>(vec!["."], vec![("INPUT", "from_env")])
            .unwrap();
    assert_eq!(result.input, "from_env");
}

#[test]
fn test_positional_cannot_use_with_short() {
    // This should not compile
    // #[derive(Conf)]
    // struct Bad {
    //     #[conf(pos, short)]
    //     input: String,
    // }
}

#[test]
fn test_positional_cannot_use_with_long() {
    // This should not compile
    // #[derive(Conf)]
    // struct Bad {
    //     #[conf(pos, long)]
    //     input: String,
    // }
}
