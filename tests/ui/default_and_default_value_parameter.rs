use conf::Conf;

#[derive(Conf)]
struct Config {
    #[conf(long, default(42), default_value = "10")]
    value: i32,
}

fn main() {}
