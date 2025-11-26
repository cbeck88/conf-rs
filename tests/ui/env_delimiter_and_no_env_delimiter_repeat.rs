use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(
        repeat,
        long,
        env,
        env_delimiter = ':',
        no_env_delimiter
    )]
    pub values: Vec<String>,
}

fn main() {}
