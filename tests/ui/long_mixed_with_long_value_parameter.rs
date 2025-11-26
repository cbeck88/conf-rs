use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, long = "custom")]
    pub value: String,
}

fn main() {}
