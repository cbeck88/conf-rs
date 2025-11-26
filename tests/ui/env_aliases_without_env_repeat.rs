use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, long, env_aliases = ["VALUES_ENV"])]
    pub values: Vec<String>,
}

fn main() {}
