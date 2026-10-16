use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, default_value = "x", default_value_help_str = "first", default_help_str = "second")]
    pub value: String,
}

fn main() {}
