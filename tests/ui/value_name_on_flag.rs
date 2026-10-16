use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, value_name = "VALUE")]
    pub verbose: bool,
}

fn main() {}
