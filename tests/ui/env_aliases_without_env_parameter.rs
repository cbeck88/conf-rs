use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, env_aliases = ["ALIAS1", "ALIAS2"])]
    pub value: String,
}

fn main() {}
