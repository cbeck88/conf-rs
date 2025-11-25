use conf::Conf;
use std::ffi::OsString;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, long, env, env_delimiter = '文')]
    pub items: Vec<OsString>,
}

fn main() {}
