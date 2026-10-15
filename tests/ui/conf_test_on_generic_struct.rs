use conf::Conf;

#[derive(Conf)]
#[conf(test)]
pub struct TestedGeneric<T: std::str::FromStr<Err: std::fmt::Display>> {
    #[conf(long)]
    pub value: T,
}

fn main() {}
