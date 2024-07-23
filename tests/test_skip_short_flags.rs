mod common;
use common::*;

use conf::{Conf, ParseType};

#[derive(Conf)]
struct A {
    #[conf(short, env)]
    f: bool,

    #[conf(flatten, skip_short=['f'])]
    b1: B,

    #[conf(flatten, prefix, skip_short=['f'])]
    b2: B,
}

#[derive(Conf, Debug)]
struct B {
    #[conf(short, long)]
    flag: bool,
}

#[test]
fn test_skip_short_a_get_program_options() {
    let opts = A::get_program_options().unwrap();

    let mut iter = opts.iter();

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Flag);
    assert_eq!(opt.short_form, Some('f'));
    assert_eq!(opt.long_form.as_deref(), None);
    assert_eq!(opt.env_form.as_deref(), Some("F"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Flag);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("flag"));
    assert_eq!(opt.env_form.as_deref(), None);

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Flag);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("b2-flag"));
    assert_eq!(opt.env_form.as_deref(), None);

    assert_eq!(iter.next(), None);
}

#[test]
fn test_skip_short_b_get_program_options() {
    let opts = B::get_program_options().unwrap();

    let mut iter = opts.iter();

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Flag);
    assert_eq!(opt.short_form, Some('f'));
    assert_eq!(opt.long_form.as_deref(), Some("flag"));
    assert_eq!(opt.env_form.as_deref(), None);

    assert_eq!(iter.next(), None);
}

#[test]
fn test_skip_short_flags_parsing() {
    let a = A::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert!(!a.f);
    assert!(!a.b1.flag);
    assert!(!a.b2.flag);

    let a = A::try_parse_from::<&str, &str, &str>(vec![".", "-f"], vec![]).unwrap();
    assert!(a.f);
    assert!(!a.b1.flag);
    assert!(!a.b2.flag);

    let a = A::try_parse_from::<&str, &str, &str>(vec![".", "--flag"], vec![]).unwrap();
    assert!(!a.f);
    assert!(a.b1.flag);
    assert!(!a.b2.flag);

    let a = A::try_parse_from::<&str, &str, &str>(vec![".", "--flag", "-f"], vec![]).unwrap();
    assert!(a.f);
    assert!(a.b1.flag);
    assert!(!a.b2.flag);

    let a = A::try_parse_from::<&str, &str, &str>(vec![".", "--flag", "-f", "--b2-flag"], vec![])
        .unwrap();
    assert!(a.f);
    assert!(a.b1.flag);
    assert!(a.b2.flag);
}

#[derive(Conf, Debug)]
struct BadA {
    #[conf(short, env)]
    f: bool,

    #[conf(flatten)]
    b1: B,

    #[conf(flatten, prefix)]
    b2: B,
}

#[test]
#[should_panic(expected = "Short option names must be unique")]
fn test_skip_short_flags_parsing_expected_collision_error() {
    let _ = BadA::try_parse_from::<&str, &str, &str>(vec!["."], vec![]);
}

#[derive(Conf, Debug)]
struct BadA2 {
    #[conf(short, env)]
    f: bool,

    #[conf(flatten, skip_short = ['f', 'a'])]
    b1: B,

    #[conf(flatten, prefix, skip_short = ['f'])]
    b2: B,
}

#[test]
fn test_skip_short_flags_program_options_unknown_skip_error() {
    assert_error_contains_text!(
        BadA2::get_program_options(),
        ["When flattening B at b1, these short options were not found: ['a']"]
    );
}

#[derive(Conf, Debug)]
struct BadA3 {
    #[conf(short, env)]
    f: bool,

    #[conf(flatten, skip_short = ['f'])]
    b1: B,

    #[conf(flatten, prefix, skip_short = ['f', 'c', 'd'])]
    b2: B,
}

#[test]
fn test_skip_short_flags_program_options_unknown_skip_error2() {
    assert_error_contains_text!(
        BadA3::get_program_options(),
        ["When flattening B at b2, these short options were not found: ['c', 'd']"]
    );
}
