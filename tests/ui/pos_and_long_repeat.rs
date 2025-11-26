use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(repeat, pos, long)]
    pub values: Vec<String>,
}

fn main() {}
