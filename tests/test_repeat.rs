use conf::{Conf, ParseType};

mod common;
use common::vec_str;

#[derive(Conf)]
struct TestRepeats {
    // This is an optional param
    #[conf(short, long = "foo", env)]
    my_option: Option<String>,

    /// This is a repeat option
    #[conf(repeat, long, env)]
    my_list: Vec<String>,

    /// This is a repeat option with a custom env delimiter
    #[conf(repeat, long, env, env_delimiter = '|')]
    my_other_list: Vec<String>,
}

#[test]
fn test_repeats_get_program_options() {
    let parser_config = TestRepeats::get_parser_config().unwrap();
    let opts = TestRepeats::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();

    assert!(!parser_config.no_help_flag);
    assert!(parser_config.about.is_none());

    assert_eq!(opts.len(), 3);

    assert_eq!(opts[0].parse_type, ParseType::Parameter);
    assert_eq!(opts[0].short_form, Some('m'));
    assert_eq!(opts[0].long_form.as_deref(), Some("foo"));
    assert_eq!(opts[0].env_form.as_deref(), Some("MY_OPTION"));
    assert_eq!(opts[0].default_value, None);
    assert!(!opts[0].is_required);
    assert_eq!(opts[0].description.as_deref(), None);

    assert_eq!(opts[1].parse_type, ParseType::Repeat);
    assert_eq!(opts[1].short_form, None);
    assert_eq!(opts[1].long_form.as_deref(), Some("my-list"));
    assert_eq!(opts[1].env_form.as_deref(), Some("MY_LIST"));
    assert_eq!(opts[1].default_value, None);
    assert!(!opts[1].is_required);
    assert_eq!(
        opts[1].description.as_deref(),
        Some("This is a repeat option")
    );

    assert_eq!(opts[2].parse_type, ParseType::Repeat);
    assert_eq!(opts[2].short_form, None);
    assert_eq!(opts[2].long_form.as_deref(), Some("my-other-list"));
    assert_eq!(opts[2].env_form.as_deref(), Some("MY_OTHER_LIST"));
    assert_eq!(opts[2].default_value, None);
    assert!(!opts[2].is_required);
    assert_eq!(
        opts[2].description.as_deref(),
        Some("This is a repeat option with a custom env delimiter")
    );
}

#[test]
fn test_repeats_parsing() {
    let result = TestRepeats::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert_eq!(result.my_option, None);
    assert_eq!(result.my_list, vec_str([]));
    assert_eq!(result.my_other_list, vec_str([]));

    // Setting the repeat option once should work
    let result =
        TestRepeats::try_parse_from::<&str, &str, &str>(vec![".", "--my-list", "foo"], vec![])
            .unwrap();
    assert_eq!(result.my_option, None);
    assert_eq!(result.my_list, vec_str(["foo"]));
    assert_eq!(result.my_other_list, vec_str([]));

    // Setting the repeat option twice should work, and aggregate results correctly
    let result = TestRepeats::try_parse_from::<&str, &str, &str>(
        vec![".", "--my-list", "foo", "--my-list", "bar"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.my_option, None);
    assert_eq!(result.my_list, vec_str(["foo", "bar"]));
    assert_eq!(result.my_other_list, vec_str([]));

    // Setting another parameter in between the list flags should still work
    let result = TestRepeats::try_parse_from::<&str, &str, &str>(
        vec![".", "--my-list", "foo", "--foo=opt", "--my-list", "bar"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.my_option.as_deref(), Some("opt"));
    assert_eq!(result.my_list, vec_str(["foo", "bar"]));
    assert_eq!(result.my_other_list, vec_str([]));

    // Interleaving values of the second list should still work
    let result = TestRepeats::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--my-list",
            "foo",
            "--foo=opt",
            "--my-other-list=x",
            "--my-list",
            "bar",
            "--my-other-list",
            "y",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.my_option.as_deref(), Some("opt"));
    assert_eq!(result.my_list, vec_str(["foo", "bar"]));
    assert_eq!(result.my_other_list, vec_str(["x", "y"]));

    // Sticking commas in cli args should still work, delimiter only applies to env
    let result = TestRepeats::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--my-list",
            ",foo,blah,",
            "--foo=opt",
            "--my-other-list=x",
            "--my-list",
            "bar",
            "--my-other-list",
            "y",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.my_option.as_deref(), Some("opt"));
    assert_eq!(result.my_list, vec_str([",foo,blah,", "bar"]));
    assert_eq!(result.my_other_list, vec_str(["x", "y"]));

    // Sticking = in cli args should still work
    let result = TestRepeats::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--my-list",
            ",foo,blah,",
            "--foo=opt",
            "--my-other-list==x",
            "--my-list",
            "bar",
            "--my-other-list",
            "=y",
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(result.my_option.as_deref(), Some("opt"));
    assert_eq!(result.my_list, vec_str([",foo,blah,", "bar"]));
    assert_eq!(result.my_other_list, vec_str(["=x", "=y"]));
}

#[test]
fn test_repeats_env_parsing() {
    let result =
        TestRepeats::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_LIST", "foo")])
            .unwrap();
    assert_eq!(result.my_option, None);
    assert_eq!(result.my_list, vec_str(["foo"]));
    assert_eq!(result.my_other_list, vec_str([]));

    let result =
        TestRepeats::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_LIST", "foo,bar")])
            .unwrap();
    assert_eq!(result.my_option, None);
    assert_eq!(result.my_list, vec_str(["foo", "bar"]));
    assert_eq!(result.my_other_list, vec_str([]));

    // Trailing comma should work as expected
    let result =
        TestRepeats::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_LIST", "foo,bar,")])
            .unwrap();
    assert_eq!(result.my_option, None);
    assert_eq!(result.my_list, vec_str(["foo", "bar", ""]));
    assert_eq!(result.my_other_list, vec_str([]));

    // Double comma should work as expected
    let result =
        TestRepeats::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_LIST", "foo,,bar,")])
            .unwrap();
    assert_eq!(result.my_option, None);
    assert_eq!(result.my_list, vec_str(["foo", "", "bar", ""]));
    assert_eq!(result.my_other_list, vec_str([]));

    // The | custom delimiter should work as expected
    let result = TestRepeats::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("MY_LIST", "foo,bar"), ("MY_OTHER_LIST", "baz,|qux")],
    )
    .unwrap();
    assert_eq!(result.my_option, None);
    assert_eq!(result.my_list, vec_str(["foo", "bar"]));
    assert_eq!(result.my_other_list, vec_str(["baz,", "qux"]));

    // CLI should shadow env
    let result = TestRepeats::try_parse_from::<&str, &str, &str>(
        vec![".", "--my-list=,foo"],
        vec![("MY_LIST", "foo,bar"), ("MY_OTHER_LIST", "baz,|qux")],
    )
    .unwrap();
    assert_eq!(result.my_option, None);
    assert_eq!(result.my_list, vec_str([",foo"]));
    assert_eq!(result.my_other_list, vec_str(["baz,", "qux"]));

    // CLI should shadow env
    let result = TestRepeats::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--my-list=,foo",
            "--my-other-list",
            "baz|qux",
            "--my-other-list",
            "=fiddle",
        ],
        vec![("MY_LIST", "foo,bar"), ("MY_OTHER_LIST", "baz,|qux")],
    )
    .unwrap();
    assert_eq!(result.my_option, None);
    assert_eq!(result.my_list, vec_str([",foo"]));
    assert_eq!(result.my_other_list, vec_str(["baz|qux", "=fiddle"]));
}

#[derive(Conf)]
struct TestRepeats2 {
    /// This is a repeat option with no env delimiter
    #[conf(repeat, long = "my", env, no_env_delimiter)]
    my_list: Vec<String>,
}

#[test]
fn test_repeats2_get_program_options() {
    let parser_config = TestRepeats2::get_parser_config().unwrap();
    let opts = TestRepeats2::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();

    assert!(!parser_config.no_help_flag);
    assert!(parser_config.about.is_none());

    assert_eq!(opts.len(), 1);

    assert_eq!(opts[0].parse_type, ParseType::Repeat);
    assert_eq!(opts[0].short_form, None);
    assert_eq!(opts[0].long_form.as_deref(), Some("my"));
    assert_eq!(opts[0].env_form.as_deref(), Some("MY_LIST"));
    assert_eq!(opts[0].default_value, None);
    assert!(!opts[0].is_required);
    assert_eq!(
        opts[0].description.as_deref(),
        Some("This is a repeat option with no env delimiter")
    );
}

#[test]
fn test_repeats2_parsing() {
    let result = TestRepeats2::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert_eq!(result.my_list, vec_str([]));

    let result = TestRepeats2::try_parse_from::<&str, &str, &str>(
        vec![".", "--my", "foo", "--my", "bar"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.my_list, vec_str(["foo", "bar"]));

    let result =
        TestRepeats2::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_LIST", "foo,bar")])
            .unwrap();

    assert_eq!(result.my_list, vec_str(["foo,bar"]));
}

#[derive(Conf)]
struct TestRepeatsShort {
    /// This is a repeat option with short and long
    #[conf(repeat, short, long, env)]
    items: Vec<String>,

    /// This is a repeat option with explicit short char
    #[conf(repeat, short = 'x', long)]
    extras: Vec<i32>,
}

#[test]
fn test_repeats_short_get_program_options() {
    let opts = TestRepeatsShort::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();

    assert_eq!(opts.len(), 2);

    assert_eq!(opts[0].parse_type, ParseType::Repeat);
    assert_eq!(opts[0].short_form, Some('i'));
    assert_eq!(opts[0].long_form.as_deref(), Some("items"));
    assert_eq!(opts[0].env_form.as_deref(), Some("ITEMS"));

    assert_eq!(opts[1].parse_type, ParseType::Repeat);
    assert_eq!(opts[1].short_form, Some('x'));
    assert_eq!(opts[1].long_form.as_deref(), Some("extras"));
    assert_eq!(opts[1].env_form, None);
}

#[test]
fn test_repeats_short_parsing() {
    // Using short flags
    let result = TestRepeatsShort::try_parse_from::<&str, &str, &str>(
        vec![".", "-i", "foo", "-i", "bar"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.items, vec_str(["foo", "bar"]));
    assert_eq!(result.extras, Vec::<i32>::new());

    // Mixing short and long flags
    let result = TestRepeatsShort::try_parse_from::<&str, &str, &str>(
        vec![".", "-i", "foo", "--items", "bar", "-i", "baz"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.items, vec_str(["foo", "bar", "baz"]));

    // Using explicit short char
    let result = TestRepeatsShort::try_parse_from::<&str, &str, &str>(
        vec![".", "-x", "1", "-x", "2", "-x", "3"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.extras, vec![1, 2, 3]);

    // Mixing both repeat fields with short flags
    let result = TestRepeatsShort::try_parse_from::<&str, &str, &str>(
        vec![".", "-i", "a", "-x", "10", "-i", "b", "-x", "20"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.items, vec_str(["a", "b"]));
    assert_eq!(result.extras, vec![10, 20]);

    // Short flag with = syntax
    let result =
        TestRepeatsShort::try_parse_from::<&str, &str, &str>(vec![".", "-i=foo", "-i=bar"], vec![])
            .unwrap();
    assert_eq!(result.items, vec_str(["foo", "bar"]));
}

#[test]
fn test_repeats_short_with_env() {
    // Env should work when short is defined
    let result =
        TestRepeatsShort::try_parse_from::<&str, &str, &str>(vec!["."], vec![("ITEMS", "x,y,z")])
            .unwrap();
    assert_eq!(result.items, vec_str(["x", "y", "z"]));

    // CLI short should shadow env
    let result = TestRepeatsShort::try_parse_from::<&str, &str, &str>(
        vec![".", "-i", "cli"],
        vec![("ITEMS", "env1,env2")],
    )
    .unwrap();
    assert_eq!(result.items, vec_str(["cli"]));
}
