use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, flatten)]
    pub items: Vec<String>,
}

fn main() {}
