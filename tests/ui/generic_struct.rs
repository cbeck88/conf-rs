use conf::Conf;
use std::marker::PhantomData;

#[derive(Conf)]
pub struct TypeGeneric<T: std::str::FromStr> {
    #[conf(long)]
    pub value: T,
}

#[derive(Conf)]
pub struct LifetimeGeneric<'a> {
    #[conf(long)]
    pub value: u32,
    #[conf(default, default_help_str = "none")]
    pub marker: PhantomData<&'a ()>,
}

#[derive(Conf)]
pub struct ConstGeneric<const N: usize> {
    #[conf(long)]
    pub value: u32,
}

fn main() {}
