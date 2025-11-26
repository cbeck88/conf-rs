use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(flag, long, long = "custom")]
    pub enabled: bool,
}

fn main() {}
