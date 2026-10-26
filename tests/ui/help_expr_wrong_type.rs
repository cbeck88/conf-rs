use conf::Conf;

#[derive(Conf)]
struct Config {
    #[conf(long, help = 5)]
    port: Option<u16>,
}

fn main() {}
