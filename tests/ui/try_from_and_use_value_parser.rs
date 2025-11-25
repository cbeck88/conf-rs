use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, env, use_value_parser)]
    #[serde(try_from = "String")]
    pub value: String,
}

fn main() {}
