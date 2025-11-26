use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, long, env, env)]
    pub items: Vec<String>,
}

fn main() {}
