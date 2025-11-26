use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(flag, short, short = 'x')]
    pub enabled: bool,
}

fn main() {}
