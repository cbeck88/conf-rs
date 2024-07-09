mod common;
use common::*;
use conf::Conf;

#[derive(Conf, Debug, PartialEq, Eq)]
#[conf(one_of_fields(a, b, c))]
struct OneOfFields {
    #[arg(short, long)]
    a: bool,
    #[arg(short, long)]
    b: Option<String>,
    #[arg(repeat, long)]
    c: Vec<i64>,
}

#[test]
fn test_one_of_fields_parsing() {
    assert_error_contains_text!(
        OneOfFields::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["Too few arguments", "--a", "--b", "--c"]
    );

    let result = OneOfFields::try_parse_from::<&str, &str, &str>(vec![".", "-a"], vec![]).unwrap();
    assert!(result.a);
    assert!(result.b.is_none());
    assert!(result.c.is_empty());

    let result =
        OneOfFields::try_parse_from::<&str, &str, &str>(vec![".", "-b=foo"], vec![]).unwrap();
    assert!(!result.a);
    assert_eq!(result.b.as_deref(), Some("foo"));
    assert!(result.c.is_empty());

    assert_error_contains_text!(
        OneOfFields::try_parse_from::<&str, &str, &str>(vec![".", "-b=foo", "-a"], vec![]),
        ["Too many arguments", "--a", "--b"],
        not["-c"]
    );

    let result =
        OneOfFields::try_parse_from::<&str, &str, &str>(vec![".", "--c", "19"], vec![]).unwrap();
    assert!(!result.a);
    assert_eq!(result.b.as_deref(), None);
    assert_eq!(result.c, vec![19]);

    let result = OneOfFields::try_parse_from::<&str, &str, &str>(
        vec![".", "--c", "19", "--c", "-45"],
        vec![],
    )
    .unwrap();
    assert!(!result.a);
    assert_eq!(result.b.as_deref(), None);
    assert_eq!(result.c, vec![19, -45]);

    assert_error_contains_text!(
        OneOfFields::try_parse_from::<&str, &str, &str>(
            vec![".", "--c", "19", "--c", "-45", "-a"],
            vec![]
        ),
        ["Too many arguments", "--c", "-a"],
        not["-b"]
    );
    assert_error_contains_text!(
        OneOfFields::try_parse_from::<&str, &str, &str>(
            vec![".", "--c", "19", "--c", "-45", "-a", "-b", "foo"],
            vec![]
        ),
        ["Too many arguments", "--c", "--a", "--b"]
    );
}

#[derive(Conf, Debug, PartialEq, Eq)]
#[conf(one_of_fields(a, b), one_of_fields(b, c))]
struct TwoOneOfFields {
    #[arg(short)]
    a: bool,
    #[arg(short)]
    b: Option<String>,
    #[arg(repeat, long)]
    c: Vec<i64>,
}

#[test]
fn test_two_one_of_fields_parsing() {
    assert_error_contains_text!(
        TwoOneOfFields::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["Too few arguments", "-a", "-b", "--c"]
    );
    assert_error_contains_text!(
        TwoOneOfFields::try_parse_from::<&str, &str, &str>(vec![".", "-a"], vec![]),
        ["Too few arguments", "-b", "--c"],
        not["-a"]
    );

    let result =
        TwoOneOfFields::try_parse_from::<&str, &str, &str>(vec![".", "-b=foo"], vec![]).unwrap();
    assert!(!result.a);
    assert_eq!(result.b.as_deref(), Some("foo"));
    assert!(result.c.is_empty());

    let result =
        TwoOneOfFields::try_parse_from::<&str, &str, &str>(vec![".", "-a", "--c", "-4"], vec![])
            .unwrap();
    assert!(result.a);
    assert!(result.b.is_none());
    assert_eq!(result.c, vec![-4]);

    assert_error_contains_text!(
        TwoOneOfFields::try_parse_from::<&str, &str, &str>(vec![".", "-b=foo", "-a"], vec![]),
        ["Too many arguments", "-b", "-a"],
        not["-c"]
    );
    assert_error_contains_text!(
        TwoOneOfFields::try_parse_from::<&str, &str, &str>(vec![".", "--c", "19"], vec![]),
        ["Too few arguments", "-b", "-a"],
        not["-c"]
    );
    assert_error_contains_text!(
        TwoOneOfFields::try_parse_from::<&str, &str, &str>(
            vec![".", "--c", "19", "--c", "-45"],
            vec![]
        ),
        ["Too few arguments", "-b", "-a"],
        not["-c"]
    );

    let result = TwoOneOfFields::try_parse_from::<&str, &str, &str>(
        vec![".", "--c", "19", "--c", "-45", "-a"],
        vec![],
    )
    .unwrap();
    assert!(result.a);
    assert_eq!(result.b.as_deref(), None);
    assert_eq!(result.c, vec![19, -45]);

    assert_error_contains_text!(
        TwoOneOfFields::try_parse_from::<&str, &str, &str>(
            vec![".", "--c", "19", "--c", "-45", "-a", "-b", "foo"],
            vec![]
        ),
        ["Too many arguments", "-b", "-a", "--c"]
    );
}

#[derive(Conf, Debug, PartialEq, Eq)]
#[conf(one_of_fields(d, e, f))]
struct OneOfFlattenedFields {
    #[conf(flatten, prefix, skip_short=['a', 'b'])]
    d: Option<OneOfFields>,
    #[conf(flatten, prefix, skip_short=['a', 'b'])]
    e: Option<OneOfFields>,
    #[conf(flatten, prefix)]
    f: Option<TwoOneOfFields>,
}

#[test]
fn test_one_of_flattened_fields_parsing() {
    assert_error_contains_text!(OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["Too few arguments", "OneOfFlattenedFields", "Argument group 'd'", "Argument group 'e'", "Argument group 'f'"],
        not [" OneOfFields", "TwoOneOfFields @ .f"]);
    assert_error_contains_text!(
        OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(vec![".", "-a"], vec![]),
        ["Too few arguments", "TwoOneOfFields @ .f", "'-b'", "'--f-c'", "because '-a'"],
        not ["OneOfFlattenedFields", "--c"]
    );

    assert_error_contains_text!(
        OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(vec![".", "--f-c=9"], vec![]),
        ["Too few arguments", "TwoOneOfFields @ .f", "'-b'", "'-a'", "because '--f-c'"],
        not ["OneOfFlattenedFields", "--c"]
    );

    let result =
        OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(vec![".", "-b=foo"], vec![])
            .unwrap();
    assert_eq!(result.d, None);
    assert_eq!(result.e, None);
    let f = result.f.as_ref().unwrap();
    assert!(!f.a);
    assert_eq!(f.b.as_deref(), Some("foo"));
    assert!(f.c.is_empty());

    let result = OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "-a", "--f-c", "-4"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.d, None);
    assert_eq!(result.e, None);
    let f = result.f.as_ref().unwrap();
    assert!(f.a);
    assert!(f.b.is_none());
    assert_eq!(f.c, vec![-4]);

    assert_error_contains_text!(
        OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
            vec![".", "-a", "--f-c", "-4", "-b=foo"],
            vec![]
        ),
        [
            "Too many arguments",
            "TwoOneOfFields @ .f",
            "'-a'",
            "'-b'",
            "'--f-c'"
        ],
        not["OneOfFlattenedFields"]
    );
    assert_error_contains_text!(OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "-a", "-b=foo"],
        vec![]
    ),
        ["Too many arguments", "TwoOneOfFields @ .f", "'-a'", "'-b'"],
        not ["OneOfFlattenedFields", "--f-c"]
    );

    let result =
        OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(vec![".", "--d-a"], vec![])
            .unwrap();
    assert_eq!(result.f, None);
    assert_eq!(result.e, None);
    let d = result.d.as_ref().unwrap();
    assert!(d.a);
    assert_eq!(d.b.as_deref(), None);
    assert!(d.c.is_empty());

    assert_error_contains_text!(OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "--d-a", "--d-b=foo"],
        vec![]
    ),
        ["Too many arguments", "OneOfFields @ .d", "--d-a", "--d-b"],
        not [" -a", " -b", "--d-c", "@ .f", "default value"]
    );

    assert_error_contains_text!(OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "--d-a", "--d-c=9"],
        vec![]
    ),
        ["Too many arguments", "OneOfFields @ .d", "--d-a", "--d-c"],
        not ["'-a'", "'-b'", "'--d-b'", "@ .f", "default value"]
    );

    let result =
        OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(vec![".", "--d-b", "foo"], vec![])
            .unwrap();
    assert_eq!(result.f, None);
    assert_eq!(result.e, None);
    let d = result.d.as_ref().unwrap();
    assert!(!d.a);
    assert_eq!(d.b.as_deref(), Some("foo"));
    assert!(d.c.is_empty());

    assert_error_contains_text!(OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "--d-a", "--e-a"],
        vec![]
    ),
        ["Too many arguments", "OneOfFlattenedFields", "--d-a", "--e-a"],
        not ["OneOfFields", "'-a'", "--d-b", "@ .f", "@ .d", "@ .e", "default value"]
    );

    assert_error_contains_text!(OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "--d-a", "--e-b", "4"],
        vec![]
    ),
        ["Too many arguments", "OneOfFlattenedFields", "--d-a", "--e-b"],
        not ["OneOfFields", "'-a'", "--e-a", "--d-b", "@ .f", "@ .d", "@ .e", "default value"]
    );

    assert_error_contains_text!(OneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "--d-a", "-b", "4"],
        vec![]
    ),
        ["Too many arguments", "OneOfFlattenedFields", "'--d-a' (part of argument group 'd')", "'-b' (part of argument group 'f')"],
        not ["OneOfFields", "'-a'", "--e-a", "--e-b", "--d-b", "@ .f", "@ .d", "@ .e", "default value"]
    );
}

#[derive(Conf, Debug, PartialEq, Eq)]
#[conf(at_least_one_of_fields(d, e, f))]
struct AtLeastOneOfFlattenedFields {
    #[conf(flatten, prefix, skip_short=['a', 'b'])]
    d: Option<OneOfFields>,
    #[conf(flatten, prefix, skip_short=['a', 'b'])]
    e: Option<OneOfFields>,
    #[conf(flatten, prefix)]
    f: Option<TwoOneOfFields>,
}

#[test]
fn test_at_least_one_of_flattened_fields_parsing() {
    assert_error_contains_text!(AtLeastOneOfFlattenedFields::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["Too few arguments", "AtLeastOneOfFlattenedFields", "Argument group 'd'", "Argument group 'e'", "Argument group 'f'"],
        not [" OneOfFields", "TwoOneOfFields @ .f"]);
    assert_error_contains_text!(
        AtLeastOneOfFlattenedFields::try_parse_from::<&str, &str, &str>(vec![".", "-a"], vec![]),
        ["Too few arguments", "TwoOneOfFields @ .f", "'-b'", "'--f-c'", "because '-a'"],
        not ["OneOfFlattenedFields", "--c"]
    );

    assert_error_contains_text!(
        AtLeastOneOfFlattenedFields::try_parse_from::<&str, &str, &str>(vec![".", "--f-c=9"], vec![]),
        ["Too few arguments", "TwoOneOfFields @ .f", "'-b'", "'-a'", "because '--f-c'"],
        not ["OneOfFlattenedFields", "--c"]
    );

    let result = AtLeastOneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "-b=foo"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.d, None);
    assert_eq!(result.e, None);
    let f = result.f.as_ref().unwrap();
    assert!(!f.a);
    assert_eq!(f.b.as_deref(), Some("foo"));
    assert!(f.c.is_empty());

    let result = AtLeastOneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "-a", "--f-c", "-4"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.d, None);
    assert_eq!(result.e, None);
    let f = result.f.as_ref().unwrap();
    assert!(f.a);
    assert!(f.b.is_none());
    assert_eq!(f.c, vec![-4]);

    assert_error_contains_text!(
        AtLeastOneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
            vec![".", "-a", "--f-c", "-4", "-b=foo"],
            vec![]
        ),
        [
            "Too many arguments",
            "TwoOneOfFields @ .f",
            "'-a'",
            "'-b'",
            "'--f-c'"
        ],
        not["OneOfFlattenedFields"]
    );
    assert_error_contains_text!(AtLeastOneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "-a", "-b=foo"],
        vec![]
    ),
        ["Too many arguments", "TwoOneOfFields @ .f", "'-a'", "'-b'"],
        not ["OneOfFlattenedFields", "--f-c"]
    );

    let result = AtLeastOneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "-a", "--f-c", "-4", "--d-b=99"],
        vec![],
    )
    .unwrap();
    let d = result.d.as_ref().unwrap();
    assert!(!d.a);
    assert_eq!(d.b.as_deref(), Some("99"));
    assert!(d.c.is_empty());
    assert_eq!(result.e, None);
    let f = result.f.as_ref().unwrap();
    assert!(f.a);
    assert!(f.b.is_none());
    assert_eq!(f.c, vec![-4]);

    assert_error_contains_text!(
        AtLeastOneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
            vec![".", "-a", "--f-c", "-4", "--d-b=99", "--e-a", "--e-c=-77"],
            vec![],
        ),
        [
            "Too many arguments",
            "constraint on OneOfFields @ .e",
            "'--e-a'",
            "'--e-c'"
        ]
    );

    let result = AtLeastOneOfFlattenedFields::try_parse_from::<&str, &str, &str>(
        vec![".", "-a", "--f-c", "-4", "--d-b=99", "--e-c=-77"],
        vec![],
    )
    .unwrap();
    let d = result.d.as_ref().unwrap();
    assert!(!d.a);
    assert_eq!(d.b.as_deref(), Some("99"));
    assert!(d.c.is_empty());
    let e = result.e.as_ref().unwrap();
    assert!(!e.a);
    assert!(e.b.is_none());
    assert_eq!(e.c, vec![-77]);
    let f = result.f.as_ref().unwrap();
    assert!(f.a);
    assert!(f.b.is_none());
    assert_eq!(f.c, vec![-4]);
}
