use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(flag, long, env, env)]
    pub enabled: bool,
}

fn main() {}
