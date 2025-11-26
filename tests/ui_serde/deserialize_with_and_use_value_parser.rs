use conf::Conf;
use serde::Deserialize;

fn custom_deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer)
}

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, env, serde(use_value_parser, deserialize_with = "custom_deserialize"))]
    pub value: String,
}

fn main() {}
