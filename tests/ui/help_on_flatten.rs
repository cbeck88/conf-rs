use conf::Conf;

#[derive(Conf)]
struct Inner {
    #[conf(long)]
    port: Option<u16>,
}

#[derive(Conf)]
struct Config {
    #[conf(flatten, help = "Inner options")]
    inner: Inner,
}

fn main() {}
