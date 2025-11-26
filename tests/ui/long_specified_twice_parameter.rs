use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, long)]
    pub value: String,
}

fn main() {}
