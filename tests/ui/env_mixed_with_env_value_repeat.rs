use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, long, env, env = "CUSTOM_NAME")]
    pub items: Vec<String>,
}

fn main() {}
