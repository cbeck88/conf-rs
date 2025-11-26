use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, short, short)]
    pub items: Vec<String>,
}

fn main() {}
