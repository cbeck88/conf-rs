use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, repeat)]
    pub values: Vec<String>,
}

fn main() {}
