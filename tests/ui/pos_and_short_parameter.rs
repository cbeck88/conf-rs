use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(parameter, pos, short)]
    pub value: String,
}

fn main() {}
