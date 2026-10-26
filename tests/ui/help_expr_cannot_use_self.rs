use conf::Conf;

#[derive(Conf)]
struct Config {
    // The expression goes in its own fn, so it can't see `Self`
    #[conf(long, help = Self::HELP)]
    port: Option<u16>,
}

impl Config {
    const HELP: &'static str = "Port";
}

fn main() {}
