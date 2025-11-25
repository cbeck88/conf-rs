use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, default_value = "first", default_value = "second")]
    pub value: String,
}

fn main() {}
