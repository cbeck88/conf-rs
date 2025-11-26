use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(short, short)]
    pub value: String,
}

fn main() {}
