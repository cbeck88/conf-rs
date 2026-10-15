use conf::Conf;

#[derive(Conf)]
pub struct SkipWithLong {
    #[conf(long, skip)]
    pub value: u32,
}

#[derive(Conf)]
pub struct SkipInSeparateAttribute {
    #[conf(skip)]
    #[conf(env)]
    pub value: u32,
}

#[derive(Conf)]
pub struct SkipTwice {
    #[conf(skip, skip = 1)]
    pub value: u32,
}

fn main() {}
