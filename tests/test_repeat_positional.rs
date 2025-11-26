mod common;

use conf::Conf;

#[derive(Conf, Debug)]
struct SingleRepeatPositional {
    /// Files to process
    #[conf(repeat, pos)]
    files: Vec<String>,
}

#[derive(Conf, Debug)]
struct RepeatPositionalWithRegularPositional {
    /// Command to run
    #[conf(pos)]
    command: String,
    /// Arguments to pass
    #[conf(repeat, pos, allow_hyphen_values)]
    args: Vec<String>,
}

#[derive(Conf, Debug)]
struct RepeatPositionalOptional {
    /// Output file (optional)
    #[conf(pos)]
    output: Option<String>,
    /// Input files
    #[conf(repeat, pos)]
    inputs: Vec<String>,
}

#[test]
fn test_single_repeat_positional() {
    // Test with no arguments
    let result =
        SingleRepeatPositional::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert_eq!(result.files.len(), 0);

    // Test with one argument
    let result =
        SingleRepeatPositional::try_parse_from::<&str, &str, &str>(vec![".", "file1.txt"], vec![])
            .unwrap();
    assert_eq!(result.files, vec!["file1.txt"]);

    // Test with multiple arguments
    let result = SingleRepeatPositional::try_parse_from::<&str, &str, &str>(
        vec![".", "file1.txt", "file2.txt", "file3.txt"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.files, vec!["file1.txt", "file2.txt", "file3.txt"]);
}

#[test]
fn test_repeat_positional_with_regular_positional() {
    // Test with command only (no args)
    let result = RepeatPositionalWithRegularPositional::try_parse_from::<&str, &str, &str>(
        vec![".", "echo"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.command, "echo");
    assert_eq!(result.args.len(), 0);

    // Test with command and one arg
    let result = RepeatPositionalWithRegularPositional::try_parse_from::<&str, &str, &str>(
        vec![".", "echo", "hello"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.command, "echo");
    assert_eq!(result.args, vec!["hello"]);

    // Test with command and multiple args
    let result = RepeatPositionalWithRegularPositional::try_parse_from::<&str, &str, &str>(
        vec![".", "ls", "-l", "-a", "/tmp"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.command, "ls");
    assert_eq!(result.args, vec!["-l", "-a", "/tmp"]);
}

#[test]
fn test_repeat_positional_with_optional_positional() {
    // Test with no output, no inputs
    let result =
        RepeatPositionalOptional::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert_eq!(result.output, None);
    assert_eq!(result.inputs.len(), 0);

    // Test with output only
    let result = RepeatPositionalOptional::try_parse_from::<&str, &str, &str>(
        vec![".", "output.txt"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.output, Some("output.txt".to_string()));
    assert_eq!(result.inputs.len(), 0);

    // Test with output and one input
    let result = RepeatPositionalOptional::try_parse_from::<&str, &str, &str>(
        vec![".", "output.txt", "input1.txt"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.output, Some("output.txt".to_string()));
    assert_eq!(result.inputs, vec!["input1.txt"]);

    // Test with output and multiple inputs
    let result = RepeatPositionalOptional::try_parse_from::<&str, &str, &str>(
        vec![".", "output.txt", "input1.txt", "input2.txt"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.output, Some("output.txt".to_string()));
    assert_eq!(result.inputs, vec!["input1.txt", "input2.txt"]);
}

#[derive(Conf, Debug)]
struct RepeatPositionalWithEnv {
    /// Files to process
    #[conf(repeat, pos, env = "FILES")]
    files: Vec<String>,
}

#[test]
fn test_repeat_positional_with_env() {
    // Test with env only
    let result = RepeatPositionalWithEnv::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("FILES", "file1.txt,file2.txt")],
    )
    .unwrap();
    assert_eq!(result.files, vec!["file1.txt", "file2.txt"]);

    // Test with positional args (should override env)
    let result = RepeatPositionalWithEnv::try_parse_from::<&str, &str, &str>(
        vec![".", "override1.txt", "override2.txt"],
        vec![("FILES", "file1.txt,file2.txt")],
    )
    .unwrap();
    assert_eq!(result.files, vec!["override1.txt", "override2.txt"]);
}

#[derive(Conf, Debug)]
struct RepeatPositionalWithValueParser {
    /// Numbers to process
    #[conf(repeat, pos)]
    numbers: Vec<i32>,
}

#[test]
fn test_repeat_positional_with_value_parser() {
    // Test with multiple numbers
    let result = RepeatPositionalWithValueParser::try_parse_from::<&str, &str, &str>(
        vec![".", "1", "2", "3", "42"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.numbers, vec![1, 2, 3, 42]);

    // Test with negative numbers (allow_negative_numbers should be automatic for signed types)
    let result = RepeatPositionalWithValueParser::try_parse_from::<&str, &str, &str>(
        vec![".", "-5", "10", "-15"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.numbers, vec![-5, 10, -15]);
}
