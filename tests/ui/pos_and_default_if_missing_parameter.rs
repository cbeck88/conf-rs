use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(parameter, pos, default_if_missing = "fallback")]
    pub value: String,
}

fn main() {}
