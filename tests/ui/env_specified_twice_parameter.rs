use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, env, env)]
    pub value: String,
}

fn main() {}
