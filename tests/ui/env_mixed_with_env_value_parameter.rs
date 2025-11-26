use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, env, env = "CUSTOM_NAME")]
    pub value: String,
}

fn main() {}
