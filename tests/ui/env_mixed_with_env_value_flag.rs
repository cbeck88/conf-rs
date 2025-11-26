use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(flag, long, env, env = "CUSTOM_NAME")]
    pub enabled: bool,
}

fn main() {}
