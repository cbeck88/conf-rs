use conf::Conf;
use std::marker::PhantomData;

#[derive(Conf)]
pub struct LifetimeGeneric<'a> {
    #[conf(long)]
    pub value: u32,
    #[conf(default, default_value_help_str = "none")]
    pub marker: PhantomData<&'a ()>,
}

// Only the lifetime is an error, not the type param
#[derive(Conf)]
pub struct MixedGeneric<'a, T: std::str::FromStr> {
    #[conf(long)]
    pub value: T,
    #[conf(default, default_value_help_str = "none")]
    pub marker: PhantomData<&'a ()>,
}

fn main() {}
