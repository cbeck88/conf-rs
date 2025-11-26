use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(short, short = 'x')]
    pub value: String,
}

fn main() {}
