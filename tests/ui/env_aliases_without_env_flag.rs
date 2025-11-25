use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(flag, long, env_aliases = ["ALIAS1", "ALIAS2"])]
    pub enabled: bool,
}

fn main() {}
