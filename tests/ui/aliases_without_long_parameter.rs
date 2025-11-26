use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(parameter, short, aliases = ["value", "val"])]
    pub value: String,
}

fn main() {}
