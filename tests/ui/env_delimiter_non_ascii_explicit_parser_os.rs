use conf::Conf;
use std::ffi::OsString;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, long, env, env_delimiter = '日', value_parser_os = |s: &std::ffi::OsStr| Ok::<_, String>(s.to_owned()))]
    pub items: Vec<OsString>,
}

fn main() {}
