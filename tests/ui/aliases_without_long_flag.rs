use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(flag, short, aliases = ["verbose"])]
    pub verbose: bool,
}

fn main() {}
