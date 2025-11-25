use conf::Conf;

fn custom_deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer)
}

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, env, use_value_parser)]
    #[serde(deserialize_with = "custom_deserialize")]
    pub value: String,
}

fn main() {}
