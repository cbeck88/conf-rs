#![cfg(feature = "serde")]

use conf::{Conf, ConfSerde, Subcommands, SubcommandsSerde};
use serde::Deserialize;
use serde_json::json;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct SerdeGeneric<T>
where
    T: FromStr<Err: Display> + for<'de> Deserialize<'de>,
{
    #[arg(long)]
    pub value: T,
    #[arg(long)]
    pub other: Option<T>,
}

#[test]
fn test_serde_generic() {
    let s = SerdeGeneric::<u32>::conf_builder()
        .args(["prog", "--other", "3"])
        .doc("doc", json!({ "value": 5 }))
        .try_parse()
        .unwrap();
    assert_eq!(s.value, 5);
    assert_eq!(s.other, Some(3));
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct SerdeConstGeneric<const N: usize> {
    #[arg(long, default(N))]
    pub value: usize,
}

#[test]
fn test_serde_const_generic() {
    let s = SerdeConstGeneric::<4>::conf_builder()
        .args(["prog"])
        .doc("doc", json!({}))
        .try_parse()
        .unwrap();
    assert_eq!(s.value, 4);
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct Inner {
    #[arg(long)]
    pub x: u32,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct SerdeFlatten<C: ConfSerde> {
    #[arg(flatten)]
    pub inner: C,
}

#[test]
fn test_serde_flatten_generic() {
    let s = SerdeFlatten::<Inner>::conf_builder()
        .args(["prog"])
        .doc("doc", json!({ "inner": { "x": 7 } }))
        .try_parse()
        .unwrap();
    assert_eq!(s.inner.x, 7);
}

fn parse_doubled<'de, D: serde::Deserializer<'de>, T>(d: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + std::ops::Add<Output = T> + Copy,
{
    let t = T::deserialize(d)?;
    Ok(t + t)
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct SerdeDeserializeWith<T>
where
    T: FromStr<Err: Display> + for<'de> Deserialize<'de> + std::ops::Add<Output = T> + Copy,
{
    #[arg(long, serde(deserialize_with = "parse_doubled"))]
    pub value: T,
}

#[test]
fn test_serde_deserialize_with_generic() {
    let s = SerdeDeserializeWith::<u32>::conf_builder()
        .args(["prog"])
        .doc("doc", json!({ "value": 5 }))
        .try_parse()
        .unwrap();
    assert_eq!(s.value, 10);
}

#[derive(Subcommands, Debug)]
#[conf(serde)]
pub enum Cmd {
    Go(Inner),
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct SerdeSubcommands<S: SubcommandsSerde> {
    #[arg(subcommands)]
    pub cmd: S,
}

#[test]
fn test_serde_subcommands_generic() {
    let s = SerdeSubcommands::<Cmd>::conf_builder()
        .args(["prog", "go"])
        .doc("doc", json!({ "go": { "x": 3 } }))
        .try_parse()
        .unwrap();
    let Cmd::Go(inner) = s.cmd;
    assert_eq!(inner.x, 3);
}

// Named-field variants of a generic enum, with serde
#[derive(Subcommands, Debug)]
#[conf(serde)]
pub enum SerdeNamedGeneric<T, U>
where
    T: FromStr<Err: Display> + for<'de> Deserialize<'de>,
    U: FromStr<Err: Display> + for<'de> Deserialize<'de>,
{
    Both {
        #[arg(long)]
        a: T,
        #[arg(long)]
        b: U,
    },
    OnlyU {
        #[arg(long)]
        b: U,
    },
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct SerdeNamedGenericCli {
    #[arg(subcommands)]
    pub cmd: SerdeNamedGeneric<u32, String>,
}

#[test]
fn test_serde_named_field_variants_generic() {
    let s = SerdeNamedGenericCli::conf_builder()
        .args(["prog", "both", "--b", "x"])
        .doc("doc", json!({ "both": { "a": 5 } }))
        .try_parse()
        .unwrap();
    let SerdeNamedGeneric::Both { a, b } = s.cmd else {
        panic!("{s:?}")
    };
    assert_eq!((a, b.as_str()), (5, "x"));

    let s = SerdeNamedGenericCli::conf_builder()
        .args(["prog", "only-u"])
        .doc("doc", json!({ "only_u": { "b": "y" } }))
        .try_parse()
        .unwrap();
    let SerdeNamedGeneric::OnlyU { b } = s.cmd else {
        panic!("{s:?}")
    };
    assert_eq!(b, "y");
}
