use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(
        long,
        env,
        value_parser = |s: &str| Ok::<_, String>(s.to_owned()),
        value_parser = |s: &str| Ok::<_, String>(s.to_uppercase())
    )]
    pub value: String,
}

fn main() {}
