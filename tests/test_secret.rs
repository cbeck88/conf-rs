#![allow(dead_code)]

use conf::{Conf, ParseType};

#[derive(Conf)]
struct A {
    #[conf(env, secret)]
    a: String,

    #[conf(flatten)]
    b: B,

    #[conf(flatten)]
    c: C,
}

#[derive(Conf)]
struct B {
    #[conf(env)]
    d: String,
    #[conf(long, env, secret = false)]
    e: String,
}

#[derive(Conf)]
struct C {
    #[conf(env, secret = false)]
    f: String,
    #[conf(env, secret = true)]
    g: String,
}

#[test]
fn test_secret_a_get_program_options() {
    let opts = A::get_program_options().unwrap();

    let mut iter = opts.iter();

    // a is a secret because most specific marking is struct A
    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.env_form.as_deref(), Some("A"));
    assert_eq!(opt.secret, Some(true));
    assert!(opt.is_secret());

    // d is a secret because most specific marking is struct A
    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.env_form.as_deref(), Some("D"));
    assert_eq!(opt.secret, None);
    assert!(!opt.is_secret());

    // e is not a secret because most specific marking is field e
    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.env_form.as_deref(), Some("E"));
    assert_eq!(opt.secret, Some(false));
    assert!(!opt.is_secret());

    // f is not a secret because most specific marking is struct C
    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.env_form.as_deref(), Some("F"));
    assert_eq!(opt.secret, Some(false));
    assert!(!opt.is_secret());

    // g is a secret because most specific marking is field g
    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.env_form.as_deref(), Some("G"));
    assert_eq!(opt.secret, Some(true));
    assert!(opt.is_secret());

    assert_eq!(iter.next(), None);
}

#[test]
fn test_secret_b_get_program_options() {
    let opts = B::get_program_options().unwrap();

    let mut iter = opts.iter();

    // d is unmarked, so `opt.secret` is None, and it defaults to not being a secret
    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.env_form.as_deref(), Some("D"));
    assert_eq!(opt.secret, None);
    assert!(!opt.is_secret());

    // e is not a secret because most specific marking is field e
    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.env_form.as_deref(), Some("E"));
    assert_eq!(opt.secret, Some(false));
    assert!(!opt.is_secret());

    assert_eq!(iter.next(), None);
}
