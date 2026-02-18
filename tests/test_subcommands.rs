//use assert_matches::assert_matches;
use conf::{Conf, Subcommands};

//mod common;
//use common::*;

#[derive(Conf)]
struct AConfig {
    #[arg(short)]
    f: bool,

    #[conf(subcommands)]
    command: Command,
}

#[derive(Subcommands)]
enum Command {
    GiveStick(GiveStickConfig),
    DontGiveStick(DontGiveStickConfig),
}

#[derive(Conf, Debug)]
struct GiveStickConfig {
    #[conf(long)]
    times: u16,
}

#[derive(Conf)]
struct DontGiveStickConfig {
    #[conf(short)]
    o: bool,
}

#[test]
fn test_required_subcommands() {
    let result = AConfig::try_parse_from::<&str, &str, &str>(vec!["."], vec![]);
    assert!(result.is_err());

    let result =
        AConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick"], vec![]).unwrap();

    assert!(!result.f);
    let Command::DontGiveStick(result) = result.command else {
        panic!("Unexpected enum val")
    };
    assert!(!result.o);

    let result =
        AConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick", "-o"], vec![])
            .unwrap();

    assert!(!result.f);
    let Command::DontGiveStick(result) = result.command else {
        panic!("Unexpected enum val")
    };
    assert!(result.o);

    let result =
        AConfig::try_parse_from::<&str, &str, &str>(vec![".", "-o", "dont-give-stick"], vec![]);
    assert!(result.is_err());

    let result = AConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-o", "dont-give-stick", "-f"],
        vec![],
    );
    assert!(result.is_err());

    let result =
        AConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick", "-f"], vec![]);
    assert!(result.is_err());

    let result =
        AConfig::try_parse_from::<&str, &str, &str>(vec![".", "-f", "dont-give-stick"], vec![])
            .unwrap();

    assert!(result.f);
    let Command::DontGiveStick(result) = result.command else {
        panic!("Unexpected enum val")
    };
    assert!(!result.o);

    let result = AConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-f", "dont-give-stick", "-o"],
        vec![],
    )
    .unwrap();

    assert!(result.f);
    let Command::DontGiveStick(result) = result.command else {
        panic!("Unexpected enum val")
    };
    assert!(result.o);

    let result = AConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-f", "give-stick", "--times=16"],
        vec![],
    )
    .unwrap();

    assert!(result.f);
    let Command::GiveStick(result) = result.command else {
        panic!("Unexpected enum val")
    };
    assert_eq!(result.times, 16);

    let result = AConfig::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "-f",
            "give-stick",
            "--times=16",
            "dont-give-stick",
            "-o",
        ],
        vec![],
    );
    assert!(result.is_err());
}

// Same thing but now subcommands are optional
#[derive(Conf)]
struct BConfig {
    #[arg(short)]
    f: bool,

    #[conf(subcommands)]
    command: Option<Command>,
}

#[test]
fn test_optional_subcommands() {
    let result = BConfig::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert!(!result.f);
    assert!(result.command.is_none());

    let result = BConfig::try_parse_from::<&str, &str, &str>(vec![".", "-f"], vec![]).unwrap();
    assert!(result.f);
    assert!(result.command.is_none());

    let result = BConfig::try_parse_from::<&str, &str, &str>(vec![".", "-o"], vec![]);
    assert!(result.is_err());

    let result =
        BConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick"], vec![]).unwrap();

    assert!(!result.f);
    let Command::DontGiveStick(result) = result.command.unwrap() else {
        panic!("Unexpected enum val")
    };
    assert!(!result.o);

    let result =
        BConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick", "-o"], vec![])
            .unwrap();

    assert!(!result.f);
    let Command::DontGiveStick(result) = result.command.unwrap() else {
        panic!("Unexpected enum val")
    };
    assert!(result.o);

    let result =
        BConfig::try_parse_from::<&str, &str, &str>(vec![".", "-o", "dont-give-stick"], vec![]);
    assert!(result.is_err());

    let result = BConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-o", "dont-give-stick", "-f"],
        vec![],
    );
    assert!(result.is_err());

    let result =
        BConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick", "-f"], vec![]);
    assert!(result.is_err());

    let result =
        BConfig::try_parse_from::<&str, &str, &str>(vec![".", "-f", "dont-give-stick"], vec![])
            .unwrap();

    assert!(result.f);
    let Command::DontGiveStick(result) = result.command.unwrap() else {
        panic!("Unexpected enum val")
    };
    assert!(!result.o);

    let result = BConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-f", "dont-give-stick", "-o"],
        vec![],
    )
    .unwrap();

    assert!(result.f);
    let Command::DontGiveStick(result) = result.command.unwrap() else {
        panic!("Unexpected enum val")
    };
    assert!(result.o);

    let result = BConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-f", "give-stick", "--times=16"],
        vec![],
    )
    .unwrap();

    assert!(result.f);
    let Command::GiveStick(result) = result.command.unwrap() else {
        panic!("Unexpected enum val")
    };
    assert_eq!(result.times, 16);

    let result = BConfig::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "-f",
            "give-stick",
            "--times=16",
            "dont-give-stick",
            "-o",
        ],
        vec![],
    );
    assert!(result.is_err());
}

#[derive(Conf)]
struct DConfig {
    #[arg(short)]
    x: bool,

    #[conf(subcommands)]
    command: DCommand,
}

#[derive(Subcommands)]
enum DCommand {
    FrozenLake(AConfig),
    WildDog(BConfig),
    Stat,
}

#[test]
fn test_nested_subcommands() {
    let result = DConfig::try_parse_from::<&str, &str, &str>(vec!["."], vec![]);
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(vec![".", "frozen-lake"], vec![]);
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "frozen-lake", "dont-give-stick"],
        vec![],
    )
    .unwrap();
    assert!(!result.x);

    let DCommand::FrozenLake(a) = result.command else {
        panic!("Unexpected enum value")
    };
    assert!(!a.f);
    let Command::DontGiveStick(r) = a.command else {
        panic!("Unexpected enum value")
    };
    assert!(!r.o);

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-x", "frozen-lake", "dont-give-stick", "-o"],
        vec![],
    )
    .unwrap();
    assert!(result.x);

    let DCommand::FrozenLake(a) = result.command else {
        panic!("Unexpected enum value")
    };
    assert!(!a.f);
    let Command::DontGiveStick(r) = a.command else {
        panic!("Unexpected enum value")
    };
    assert!(r.o);

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "frozen-lake", "-x", "dont-give-stick"],
        vec![],
    );
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "frozen-lake", "dont-give-stick", "-x"],
        vec![],
    );
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "frozen-lake", "-o", "dont-give-stick"],
        vec![],
    );
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-o", "frozen-lake", "dont-give-stick"],
        vec![],
    );
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "wild-dog", "dont-give-stick"],
        vec![],
    )
    .unwrap();
    assert!(!result.x);
    let DCommand::WildDog(b) = result.command else {
        panic!("Unexpected enum value")
    };
    assert!(!b.f);
    let Command::DontGiveStick(r) = b.command.unwrap() else {
        panic!("Unexpected enum value")
    };
    assert!(!r.o);

    let result =
        DConfig::try_parse_from::<&str, &str, &str>(vec![".", "wild-dog", "give-stick"], vec![]);
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "wild-dog", "give-stick", "--times", "9"],
        vec![],
    )
    .unwrap();
    assert!(!result.x);
    let DCommand::WildDog(b) = result.command else {
        panic!("Unexpected enum value")
    };
    assert!(!b.f);
    let Command::GiveStick(r) = b.command.unwrap() else {
        panic!("Unexpected enum value")
    };
    assert_eq!(r.times, 9);

    let result = DConfig::try_parse_from::<&str, &str, &str>(vec![".", "stat"], vec![]).unwrap();
    assert!(!result.x);
    let DCommand::Stat = result.command else {
        panic!("Unexpected enum value")
    };
}

// Test subcommands with named fields (no separate struct needed)
#[derive(Subcommands, Debug, PartialEq)]
enum NamedFieldsCommand {
    // Unit variant
    Status,
    // Variant with named fields
    Deploy {
        #[arg(long)]
        target: String,
        #[arg(long)]
        dry_run: bool,
    },
    // Another variant with named fields
    Build {
        #[arg(long, short)]
        release: bool,
        #[arg(long, default_value = "1")]
        jobs: u32,
    },
}

#[derive(Conf)]
struct NamedFieldsConfig {
    #[arg(short)]
    verbose: bool,

    #[conf(subcommands)]
    command: NamedFieldsCommand,
}

#[test]
fn test_named_fields_subcommands() {
    // Test unit variant
    let result =
        NamedFieldsConfig::try_parse_from::<&str, &str, &str>(vec![".", "status"], vec![]).unwrap();
    assert!(!result.verbose);
    assert_eq!(result.command, NamedFieldsCommand::Status);

    // Test Deploy with required --target
    let result = NamedFieldsConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "deploy", "--target", "production"],
        vec![],
    )
    .unwrap();
    assert!(!result.verbose);
    let NamedFieldsCommand::Deploy { target, dry_run } = result.command else {
        panic!("Expected Deploy variant");
    };
    assert_eq!(target, "production");
    assert!(!dry_run);

    // Test Deploy with --dry-run
    let result = NamedFieldsConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "deploy", "--target", "staging", "--dry-run"],
        vec![],
    )
    .unwrap();
    let NamedFieldsCommand::Deploy { target, dry_run } = result.command else {
        panic!("Expected Deploy variant");
    };
    assert_eq!(target, "staging");
    assert!(dry_run);

    // Test Build with defaults
    let result =
        NamedFieldsConfig::try_parse_from::<&str, &str, &str>(vec![".", "build"], vec![]).unwrap();
    let NamedFieldsCommand::Build { release, jobs } = result.command else {
        panic!("Expected Build variant");
    };
    assert!(!release);
    assert_eq!(jobs, 1);

    // Test Build with options
    let result = NamedFieldsConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-v", "build", "--release", "--jobs", "4"],
        vec![],
    )
    .unwrap();
    assert!(result.verbose);
    let NamedFieldsCommand::Build { release, jobs } = result.command else {
        panic!("Expected Build variant");
    };
    assert!(release);
    assert_eq!(jobs, 4);

    // Test Build with short option
    let result =
        NamedFieldsConfig::try_parse_from::<&str, &str, &str>(vec![".", "build", "-r"], vec![])
            .unwrap();
    let NamedFieldsCommand::Build { release, jobs } = result.command else {
        panic!("Expected Build variant");
    };
    assert!(release);
    assert_eq!(jobs, 1);

    // Test missing required field
    let result = NamedFieldsConfig::try_parse_from::<&str, &str, &str>(vec![".", "deploy"], vec![]);
    assert!(result.is_err());
}

// Test mixing named fields with single unnamed field variants
#[derive(Subcommands, Debug)]
enum MixedCommand {
    // Unit variant
    Info,
    // Single unnamed field (existing pattern)
    Run(GiveStickConfig),
    // Named fields (new pattern)
    Configure {
        #[arg(long)]
        key: String,
        #[arg(long)]
        value: String,
    },
}

#[derive(Conf)]
struct MixedConfig {
    #[conf(subcommands)]
    command: MixedCommand,
}

#[test]
fn test_mixed_subcommand_variants() {
    // Test unit variant
    let result =
        MixedConfig::try_parse_from::<&str, &str, &str>(vec![".", "info"], vec![]).unwrap();
    let MixedCommand::Info = result.command else {
        panic!("Expected Info variant");
    };

    // Test single unnamed field variant
    let result =
        MixedConfig::try_parse_from::<&str, &str, &str>(vec![".", "run", "--times", "5"], vec![])
            .unwrap();
    let MixedCommand::Run(config) = result.command else {
        panic!("Expected Run variant");
    };
    assert_eq!(config.times, 5);

    // Test named fields variant
    let result = MixedConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "configure", "--key", "foo", "--value", "bar"],
        vec![],
    )
    .unwrap();
    let MixedCommand::Configure { key, value } = result.command else {
        panic!("Expected Configure variant");
    };
    assert_eq!(key, "foo");
    assert_eq!(value, "bar");
}

// Test that generated structs don't collide with existing types in scope.
// Here we define a struct `Deploy` in scope, but the generated struct for
// the `Deploy` variant should be isolated in the anonymous const block.
struct Deploy {
    _unrelated_field: i32,
}

#[derive(Subcommands, Debug)]
enum NameCollisionCommand {
    // This variant generates a hidden struct also named `Deploy`,
    // but it should not collide with the `Deploy` struct above.
    Deploy {
        #[arg(long)]
        target: String,
    },
    Other,
}

// More interesting collision: field type references a type with same name as variant
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Target {
    pub name: String,
}

impl std::str::FromStr for Target {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Target {
            name: s.to_string(),
        })
    }
}

#[derive(Subcommands, Debug)]
enum FieldTypeCollisionCommand {
    // The variant is named `Target` and has a field of type `Target`.
    // Without name mangling, the generated struct would shadow the outer Target type.
    // The macro uses `__FieldTypeCollisionCommand_Target` as the generated struct name to avoid this.
    Target {
        #[arg(long)]
        destination: Target, // This refers to the outer Target type
    },
}

#[derive(Conf)]
struct FieldTypeCollisionConfig {
    #[conf(subcommands)]
    command: FieldTypeCollisionCommand,
}

#[derive(Conf)]
struct NameCollisionConfig {
    #[conf(subcommands)]
    command: NameCollisionCommand,
}

#[test]
fn test_generated_struct_name_isolation() {
    // Verify the outer Deploy struct still exists and is usable
    let _outer_deploy = Deploy {
        _unrelated_field: 42,
    };

    // Verify the subcommand works correctly (uses the generated inner Deploy struct)
    let result = NameCollisionConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "deploy", "--target", "production"],
        vec![],
    )
    .unwrap();
    let NameCollisionCommand::Deploy { target } = result.command else {
        panic!("Expected Deploy variant");
    };
    assert_eq!(target, "production");

    // Test the other variant too
    let result =
        NameCollisionConfig::try_parse_from::<&str, &str, &str>(vec![".", "other"], vec![])
            .unwrap();
    let NameCollisionCommand::Other = result.command else {
        panic!("Expected Other variant");
    };
}

#[test]
fn test_field_type_collision() {
    // This tests that a variant named `Target` with a field of type `Target`
    // works correctly - the field type refers to the outer Target type,
    // not the generated struct.
    let result = FieldTypeCollisionConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "target", "--destination", "my-server"],
        vec![],
    )
    .unwrap();
    let FieldTypeCollisionCommand::Target { destination } = result.command;
    // Verify we got the outer Target type with the correct value
    assert_eq!(destination.name, "my-server");
}

// Test that struct-level conf attributes are properly forwarded to generated structs.
// Also tests that error messages use the friendly display name.
#[derive(Subcommands, Debug)]
#[allow(dead_code)]
enum ConstraintCommand {
    #[conf(one_of_fields(opt_a, opt_b))]
    Choose {
        #[arg(long)]
        opt_a: Option<String>,
        #[arg(long)]
        opt_b: Option<String>,
    },
}

#[derive(Conf)]
#[allow(dead_code)]
struct ConstraintConfig {
    #[conf(subcommands)]
    command: ConstraintCommand,
}

#[test]
fn test_named_fields_display_name_in_errors() {
    // Test that error messages use "EnumName::VariantName" format
    // instead of the mangled struct name "__EnumName_ConstraintCommand"

    // Trigger a one_of_fields error by not providing either option
    let result = ConstraintConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "choose"], // missing both --opt-a and --opt-b
        vec![],
    );

    let Err(err) = result else {
        panic!("Should fail (one_of_fields constraint not satisfied)");
    };
    let error_string = err.to_string();

    // The error should mention "ConstraintCommand::Choose", not "__ConstraintCommand_Choose"
    assert!(
        error_string.contains("ConstraintCommand::Choose"),
        "Error message should contain 'ConstraintCommand::Choose', got: {error_string}"
    );
    assert!(
        !error_string.contains("__ConstraintCommand_Choose"),
        "Error message should NOT contain mangled name '__ConstraintCommand_Choose', got: {error_string}"
    );
}

// Test flattening structs that contain subcommands

#[derive(Subcommands, Debug, PartialEq)]
enum InnerCommand {
    Start,
    Stop,
}

#[derive(Conf, Debug)]
struct InnerConfig {
    #[arg(long)]
    verbose: bool,

    #[conf(subcommands)]
    command: InnerCommand,
}

#[derive(Conf, Debug)]
struct OuterConfig {
    #[arg(long)]
    global_flag: bool,

    #[conf(flatten)]
    inner: InnerConfig,
}

#[test]
fn test_flatten_with_subcommands() {
    // Test that subcommands from flattened struct work
    let result =
        OuterConfig::try_parse_from::<&str, &str, &str>(vec![".", "start"], vec![]).unwrap();
    assert!(!result.global_flag);
    assert!(!result.inner.verbose);
    assert_eq!(result.inner.command, InnerCommand::Start);

    // Test with flags at both levels
    let result = OuterConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "--global-flag", "--verbose", "stop"],
        vec![],
    )
    .unwrap();
    assert!(result.global_flag);
    assert!(result.inner.verbose);
    assert_eq!(result.inner.command, InnerCommand::Stop);
}

#[test]
fn test_flatten_with_subcommands_missing_subcommand() {
    // Test that missing subcommand produces appropriate error
    let result = OuterConfig::try_parse_from::<&str, &str, &str>(vec!["."], vec![]);
    assert!(result.is_err());
}

// Test flattening with prefix and subcommands
// Note: prefix affects long_prefix and env_prefix, but NOT subcommand names.
// This keeps the implementation simple since flattening subcommands is rare.

#[derive(Conf, Debug)]
struct PrefixedOuterConfig {
    #[arg(long)]
    global_flag: bool,

    #[conf(flatten, prefix)]
    inner: InnerConfig,
}

#[test]
fn test_flatten_with_prefix_and_subcommands() {
    // `prefix` sets long_prefix and env_prefix, but subcommands keep their original names
    // So --verbose becomes --inner-verbose, but subcommand "start" stays "start"
    let result = PrefixedOuterConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "--inner-verbose", "start"],
        vec![],
    )
    .unwrap();
    assert!(!result.global_flag);
    assert!(result.inner.verbose);
    assert_eq!(result.inner.command, InnerCommand::Start);

    // Test the other subcommand
    let result =
        PrefixedOuterConfig::try_parse_from::<&str, &str, &str>(vec![".", "stop"], vec![]).unwrap();
    assert_eq!(result.inner.command, InnerCommand::Stop);
}

// Test that colliding subcommand names from multiple flattened structs cause a runtime error

#[derive(Subcommands, Debug, PartialEq)]
enum FirstCommand {
    Start,
    Stop,
}

#[allow(dead_code)]
#[derive(Conf, Debug)]
struct FirstConfig {
    #[conf(subcommands)]
    command: FirstCommand,
}

#[derive(Subcommands, Debug, PartialEq)]
enum SecondCommand {
    Start, // Collides with FirstCommand::Start
    Restart,
}

#[allow(dead_code)]
#[derive(Conf, Debug)]
struct SecondConfig {
    #[conf(subcommands)]
    command: SecondCommand,
}

#[allow(dead_code)]
#[derive(Conf, Debug)]
struct CollidingSubcommandsConfig {
    #[conf(flatten)]
    first: FirstConfig,
    #[conf(flatten)]
    second: SecondConfig,
}

#[test]
#[should_panic(expected = "command name `start` is duplicated")]
fn test_colliding_subcommands_panic() {
    // Clap panics at runtime when two subcommands have the same name
    let _ =
        CollidingSubcommandsConfig::try_parse_from::<&str, &str, &str>(vec![".", "start"], vec![]);
}

// Test that nested optional configs in subcommand are parsed

#[derive(Conf, Debug)]
#[allow(dead_code)]
struct Cli {
    #[conf(subcommands)]
    command: Commands,
}

#[derive(Subcommands, Debug)]
#[allow(dead_code)]
enum Commands {
    First(First),
}

#[derive(Conf, Debug)]
#[allow(dead_code)]
struct First {
    #[conf(flatten, prefix)]
    maybe: Option<NestedOptional>,
}

#[derive(Conf, Debug)]
#[allow(dead_code)]
struct NestedOptional {
    #[conf(long, env)]
    user: String,
    #[conf(long, env)]
    pass: String,
}

#[test]
fn test_nested_optionals_in_subcommand() {
    let config = Cli::parse_from::<&'static str, &'static str, &'static str>(
        vec!["my-binary", "first"],
        vec![("MAYBE_USER", "USER"), ("MAYBE_PASS", "PASS")],
    );

    let Commands::First(cmd) = config.command;
    assert_eq!(cmd.maybe.as_ref().unwrap().user, "USER");
    assert_eq!(cmd.maybe.as_ref().unwrap().pass, "PASS");

    let config = Cli::parse_from::<&'static str, &'static str, &'static str>(
        vec![
            "my-binary",
            "first",
            "--maybe-user=user",
            "--maybe-pass=pass",
        ],
        vec![],
    );

    let Commands::First(cmd) = config.command;
    assert_eq!(cmd.maybe.as_ref().unwrap().user, "user");
    assert_eq!(cmd.maybe.as_ref().unwrap().pass, "pass");
}

#[derive(Conf, Debug)]
#[allow(dead_code)]
struct NestedCli {
    #[conf(subcommands)]
    command: NestedCommands,
}

#[derive(Subcommands, Debug)]
#[allow(dead_code)]
enum NestedCommands {
    Nested(Cli),
}

#[test]
fn test_nested_optionals_in_nested_subcommand() {
    let config = NestedCli::parse_from::<&'static str, &'static str, &'static str>(
        vec!["my-binary", "nested", "first"],
        vec![("MAYBE_USER", "USER"), ("MAYBE_PASS", "PASS")],
    );

    let NestedCommands::Nested(cli) = config.command;
    let Commands::First(cmd) = cli.command;

    assert_eq!(cmd.maybe.as_ref().unwrap().user, "USER");
    assert_eq!(cmd.maybe.as_ref().unwrap().pass, "PASS");

    let config = NestedCli::parse_from::<&'static str, &'static str, &'static str>(
        vec![
            "my-binary",
            "nested",
            "first",
            "--maybe-user=user",
            "--maybe-pass=pass",
        ],
        vec![],
    );

    let NestedCommands::Nested(cli) = config.command;
    let Commands::First(cmd) = cli.command;

    assert_eq!(cmd.maybe.as_ref().unwrap().user, "user");
    assert_eq!(cmd.maybe.as_ref().unwrap().pass, "pass");
}
