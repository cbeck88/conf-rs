use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, pos, short)]
    pub values: Vec<String>,
}

fn main() {}
