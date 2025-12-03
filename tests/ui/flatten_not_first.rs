use conf::Conf;

#[derive(Conf, Debug)]
pub struct Inner {
    #[arg(long)]
    pub value: String,
}

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(long, flatten)]
    pub inner: Inner,
}

fn main() {}
