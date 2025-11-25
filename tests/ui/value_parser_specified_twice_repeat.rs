use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(
        repeat,
        long,
        env,
        value_parser = |s: &str| Ok::<_, String>(s.to_owned()),
        value_parser = |s: &str| Ok::<_, String>(s.to_uppercase())
    )]
    pub items: Vec<String>,
}

fn main() {}
