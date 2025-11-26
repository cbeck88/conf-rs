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
    #[conf(long, env, serde(deserialize_with = "custom_deserialize", try_from = "String"))]
    pub value: String,
}

fn main() {}
