use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, long, env_aliases = ["ALIAS1", "ALIAS2"])]
    pub items: Vec<String>,
}

fn main() {}
