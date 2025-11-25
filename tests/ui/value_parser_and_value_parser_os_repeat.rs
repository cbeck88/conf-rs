use conf::Conf;
use std::ffi::OsString;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(
        repeat,
        long,
        env,
        value_parser = |s: &str| Ok::<_, String>(s.to_owned()),
        value_parser_os = |s: &std::ffi::OsStr| Ok::<_, String>(s.to_owned())
    )]
    pub items: Vec<OsString>,
}

fn main() {}
