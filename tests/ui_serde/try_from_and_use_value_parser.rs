use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, env, serde(use_value_parser, try_from = "String"))]
    pub value: String,
}

fn main() {}
