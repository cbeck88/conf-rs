use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, long, long)]
    pub items: Vec<String>,
}

fn main() {}
