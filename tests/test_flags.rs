use conf::{Conf, ParseType};

#[derive(Conf)]
struct TestFlags {
    /// This is a test flag
    #[conf(long, env)]
    my_flag: bool,

    // This is an undocumented test flag
    #[conf(short, long, env)]
    my_obscure_flag: bool,
}

#[test]
fn test_flags_get_program_options() {
    let (parser_config, opts) = TestFlags::get_program_options().unwrap();

    assert!(!parser_config.no_help_flag);
    assert!(parser_config.about.is_none());

    assert_eq!(opts.len(), 2);

    assert_eq!(opts[0].parse_type, ParseType::Flag);
    assert_eq!(opts[0].short_form, None);
    assert_eq!(opts[0].long_form.as_deref(), Some("my-flag"));
    assert_eq!(opts[0].env_form.as_deref(), Some("MY_FLAG"));
    assert_eq!(opts[0].default_value, None);
    assert!(!opts[0].is_required);
    assert_eq!(opts[0].description.as_deref(), Some("This is a test flag"));

    assert_eq!(opts[1].parse_type, ParseType::Flag);
    assert_eq!(opts[1].short_form.as_deref(), Some("m"));
    assert_eq!(opts[1].long_form.as_deref(), Some("my-obscure-flag"));
    assert_eq!(opts[1].env_form.as_deref(), Some("MY_OBSCURE_FLAG"));
    assert_eq!(opts[1].default_value, None);
    assert!(!opts[1].is_required);
    assert_eq!(opts[1].description.as_deref(), None);
}

#[test]
fn test_flags_parsing() {
    let result = TestFlags::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert!(!result.my_flag);
    assert!(!result.my_obscure_flag);

    let result = TestFlags::try_parse_from::<&str, &str, &str>(vec![".", "-m"], vec![]).unwrap();
    assert!(!result.my_flag);
    assert!(result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec![".", "--my-flag"], vec![]).unwrap();
    assert!(result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec![".", "--my-obscure-flag"], vec![])
            .unwrap();
    assert!(!result.my_flag);
    assert!(result.my_obscure_flag);

    let result = TestFlags::try_parse_from::<&str, &str, &str>(
        vec![".", "--my-flag", "--my-obscure-flag"],
        vec![],
    )
    .unwrap();
    assert!(result.my_flag);
    assert!(result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec![".", "--my-flag=true"], vec![]).unwrap();
    assert!(result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec![".", "--my-flag=false"], vec![])
            .unwrap();
    assert!(!result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec![".", "--my-flag="], vec![]).unwrap();
    assert!(result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec![".", "-m=true"], vec![]).unwrap();
    assert!(!result.my_flag);
    assert!(result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec![".", "-m=false"], vec![]).unwrap();
    assert!(!result.my_flag);
    assert!(!result.my_obscure_flag);

    let result = TestFlags::try_parse_from::<&str, &str, &str>(vec![".", "-m="], vec![]).unwrap();
    assert!(!result.my_flag);
    assert!(result.my_obscure_flag);
}

#[test]
fn test_flags_env_parsing() {
    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_FLAG", "")]).unwrap();
    assert!(result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_FLAG", "1")]).unwrap();
    assert!(result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_FLAG", "0")]).unwrap();
    assert!(!result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_FLAG", "true")])
            .unwrap();
    assert!(result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_FLAG", "false")])
            .unwrap();
    assert!(!result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_FLAG", "t")]).unwrap();
    assert!(result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_FLAG", "f")]).unwrap();
    assert!(!result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_FLAG", "on")]).unwrap();
    assert!(result.my_flag);
    assert!(!result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec!["."], vec![("MY_FLAG", "off")]).unwrap();
    assert!(!result.my_flag);
    assert!(!result.my_obscure_flag);

    let result = TestFlags::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("MY_FLAG", ""), ("MY_OBSCURE_FLAG", "")],
    )
    .unwrap();
    assert!(result.my_flag);
    assert!(result.my_obscure_flag);

    let result =
        TestFlags::try_parse_from::<&str, &str, &str>(vec![".", "-m"], vec![("MY_FLAG", "")])
            .unwrap();
    assert!(result.my_flag);
    assert!(result.my_obscure_flag);
}
