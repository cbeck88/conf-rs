use conf::Conf;

#[derive(Conf)]
struct Config {
    #[conf(long, help = "first", help = "second")]
    port: Option<u16>,
}

fn main() {}
