use conf::Conf;
use std::path::PathBuf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, long, env, env_delimiter = '中')]
    pub paths: Vec<PathBuf>,
}

fn main() {}
