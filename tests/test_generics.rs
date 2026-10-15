use conf::{Conf, Subcommands};
use std::fmt::Display;
use std::str::FromStr;

fn no_env() -> Vec<(&'static str, &'static str)> {
    vec![]
}

#[derive(Conf, Debug)]
struct Simple<T: FromStr<Err: Display>> {
    #[arg(long)]
    value: T,
}

#[test]
fn test_simple() {
    let s = Simple::<u32>::try_parse_from(["prog", "--value", "5"], no_env()).unwrap();
    assert_eq!(s.value, 5);
    let s = Simple::<String>::try_parse_from(["prog", "--value", "x"], no_env()).unwrap();
    assert_eq!(s.value, "x");
    assert!(Simple::<u32>::try_parse_from(["prog", "--value", "x"], no_env()).is_err());
}

// Bounds in a where clause, Option / Vec / defaults of the generic type
#[derive(Conf, Debug)]
struct Shapes<T, U>
where
    T: FromStr + Display + Clone,
    T::Err: Display,
    U: FromStr<Err: Display>,
{
    #[arg(long)]
    opt: Option<T>,
    // The macro detects signed numbers syntactically, so it can't tell that U might be one.
    #[arg(repeat, long, allow_negative_numbers)]
    many: Vec<U>,
    #[arg(long, default_value = "7")]
    with_default: T,
    #[arg(long, default(make_default::<T>()))]
    with_default_expr: T,
    #[arg(long, value_parser = |s: &str| s.parse::<T>())]
    with_parser: T,
}

fn make_default<T: FromStr>() -> T {
    "9".parse().ok().unwrap()
}

#[test]
fn test_shapes() {
    let s = Shapes::<u8, i64>::try_parse_from(
        ["prog", "--many", "1", "--many", "-2", "--with-parser", "3"],
        no_env(),
    )
    .unwrap();
    assert_eq!(s.opt, None);
    assert_eq!(s.many, vec![1, -2]);
    assert_eq!(s.with_default, 7);
    assert_eq!(s.with_default_expr, 9);
    assert_eq!(s.with_parser, 3);
}

// Const generic params
#[derive(Conf, Debug)]
struct ConstGeneric<const N: usize> {
    #[arg(long, default(N as u32))]
    value: u32,
}

#[test]
fn test_const_generic() {
    let s = ConstGeneric::<4>::try_parse_from(["prog"], no_env()).unwrap();
    assert_eq!(s.value, 4);
}

// Flatten a generic Conf
#[derive(Conf, Debug)]
struct Inner {
    #[arg(long)]
    x: u32,
}

#[derive(Conf, Debug)]
struct Outer<C: Conf> {
    #[arg(flatten, prefix)]
    inner: C,
    #[arg(flatten, prefix = "maybe")]
    opt_inner: Option<C>,
}

#[test]
fn test_flatten_generic() {
    let s = Outer::<Inner>::try_parse_from(["prog", "--inner-x", "3"], no_env()).unwrap();
    assert_eq!(s.inner.x, 3);
    assert!(s.opt_inner.is_none());
    let s = Outer::<Inner>::try_parse_from(["prog", "--inner-x", "3", "--maybe-x", "4"], no_env())
        .unwrap();
    assert_eq!(s.opt_inner.unwrap().x, 4);
}

// Validation predicate on a generic struct
#[derive(Conf, Debug)]
#[conf(validation_predicate = check_less)]
struct Validated<T: FromStr<Err: Display> + PartialOrd> {
    #[arg(long)]
    lo: T,
    #[arg(long)]
    hi: T,
}

fn check_less<T: FromStr + PartialOrd>(v: &Validated<T>) -> Result<(), &'static str>
where
    T::Err: Display,
{
    if v.lo < v.hi {
        Ok(())
    } else {
        Err("lo must be less than hi")
    }
}

#[test]
fn test_validation_generic() {
    assert!(Validated::<i32>::try_parse_from(["prog", "--lo", "1", "--hi", "2"], no_env()).is_ok());
    assert!(
        Validated::<i32>::try_parse_from(["prog", "--lo", "2", "--hi", "1"], no_env()).is_err()
    );
}

// Generic subcommands field
#[derive(Subcommands, Debug)]
enum Cmd {
    Go(Inner),
}

#[derive(Conf, Debug)]
struct WithSub<S: Subcommands> {
    #[arg(subcommands)]
    cmd: S,
}

#[test]
fn test_subcommands_generic() {
    let s = WithSub::<Cmd>::try_parse_from(["prog", "go", "--x", "1"], no_env()).unwrap();
    let Cmd::Go(inner) = s.cmd;
    assert_eq!(inner.x, 1);
}

// Program options / help mention the generic default's Display
#[test]
fn test_help_generic_default() {
    let err = Shapes::<u8, i64>::try_parse_from(["prog", "--help"], no_env()).unwrap_err();
    let help = err.to_string();
    assert!(help.contains("[default: 9]"), "{help}");
}

// Defaults for type and const params
#[derive(Conf, Debug)]
struct Defaulted<T: FromStr<Err: Display> = u32, const N: usize = 3> {
    #[arg(long)]
    value: T,
    #[arg(long, default(N))]
    count: usize,
}

#[test]
fn test_param_defaults() {
    let s = <Defaulted>::try_parse_from(["prog", "--value", "5"], no_env()).unwrap();
    assert_eq!(s.value, 5u32);
    assert_eq!(s.count, 3);
    let s = Defaulted::<String, 8>::try_parse_from(["prog", "--value", "x"], no_env()).unwrap();
    assert_eq!(s.value, "x");
    assert_eq!(s.count, 8);
}

// The derive decides how to treat a field from how its type is written, so a field of type `T`
// is always a parameter, even when `T = bool`.
#[test]
fn test_generic_bool_is_a_parameter() {
    let s = Simple::<bool>::try_parse_from(["prog", "--value", "true"], no_env()).unwrap();
    assert!(s.value);
    assert!(Simple::<bool>::try_parse_from(["prog", "--value"], no_env()).is_err());
}

// #[conf(test)] can't go on a generic struct, but it can go on a non-generic one that uses it
// with concrete types, and then it also checks the generic struct.
#[derive(Conf, Debug)]
#[conf(test)]
struct ConcreteTop {
    #[arg(flatten)]
    shapes: Shapes<u8, i64>,
    #[arg(flatten)]
    outer: Outer<Inner>,
}

#[test]
fn test_concrete_top() {
    let s = ConcreteTop::try_parse_from(["prog", "--with-parser", "1", "--inner-x", "2"], no_env())
        .unwrap();
    assert_eq!(s.shapes.with_parser, 1);
    assert_eq!(s.outer.inner.x, 2);
}

// The example from REFERENCE_derive_conf.md
#[derive(Conf, Debug)]
pub struct ServiceConfig<T, D: Conf, const RETRIES: u32 = 3>
where
    T: FromStr,
    T::Err: Display,
{
    #[conf(long)]
    pub limit: T,

    #[conf(long, default(RETRIES))]
    pub retries: u32,

    #[conf(flatten)]
    pub db: D,
}

#[test]
fn test_reference_example() {
    let s = <ServiceConfig<u64, Inner>>::try_parse_from(
        ["prog", "--limit", "10", "--x", "1"],
        no_env(),
    )
    .unwrap();
    assert_eq!(s.limit, 10);
    assert_eq!(s.retries, 3);
    assert_eq!(s.db.x, 1);
}
