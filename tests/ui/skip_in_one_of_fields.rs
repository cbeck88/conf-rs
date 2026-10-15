use conf::Conf;

#[derive(Conf)]
#[conf(one_of_fields(a, b))]
pub struct SkipInOneOf {
    #[conf(long)]
    pub a: Option<u32>,
    #[conf(skip)]
    pub b: Option<u32>,
}

fn main() {}
