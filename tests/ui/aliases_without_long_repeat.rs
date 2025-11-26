use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, short, aliases = ["values"])]
    pub values: Vec<String>,
}

fn main() {}
