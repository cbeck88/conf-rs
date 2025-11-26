mod common;
use common::*;
use conf::Conf;

#[test]
fn test_default_help_str_overrides_default_value() {
    #[allow(unused)]
    #[derive(Conf, Debug)]
    struct TestConfig {
        /// A field with both default_value and default_help_str
        #[conf(
            long,
            default_value = "actual_default",
            default_help_str = "shown in help"
        )]
        field_with_both: String,

        /// A field with only default_value
        #[conf(long, default_value = "just_default")]
        field_with_default_only: String,

        /// A field with only default_help_str (no default_value)
        #[conf(long, default_help_str = "help string only")]
        field_help_only: Option<String>,
    }

    let opts = TestConfig::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();

    // Check field_with_both - should use default_help_str
    let opt1 = opts.iter().find(|o| o.id == "field_with_both").unwrap();
    assert_eq!(
        format_default_help_str(opt1.default_help_str).as_deref(),
        Some("shown in help")
    );

    // Check field_with_default_only - should use default_value as fallback
    let opt2 = opts
        .iter()
        .find(|o| o.id == "field_with_default_only")
        .unwrap();
    assert_eq!(
        format_default_help_str(opt2.default_help_str).as_deref(),
        Some("just_default")
    );

    // Check field_help_only - should use default_help_str
    let opt3 = opts.iter().find(|o| o.id == "field_help_only").unwrap();
    assert_eq!(
        format_default_help_str(opt3.default_help_str).as_deref(),
        Some("help string only")
    );
}

#[test]
fn test_default_help_str_in_actual_help() {
    #[allow(unused)]
    #[derive(Conf, Debug)]
    struct TestConfig {
        #[conf(long, default_value = "real_val", default_help_str = "display_val")]
        my_field: String,
    }

    let env = Default::default();
    let opts = TestConfig::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();
    let parser = TestConfig::get_parser(&env, opts).unwrap();

    let help = parser.render_clap_help();
    // The help should show the display_val, not the real_val
    assert!(
        help.contains("display_val"),
        "Help should contain default_help_str value"
    );
}

#[test]
fn test_default_value_still_works_for_initialization() {
    #[derive(Conf, Debug)]
    struct TestConfig {
        #[conf(
            long,
            default_value = "actual_default",
            default_help_str = "shown in help"
        )]
        field: String,
    }

    let result = TestConfig::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();

    // Should use the actual default_value for initialization, not the help string
    assert_eq!(result.field, "actual_default");
}
