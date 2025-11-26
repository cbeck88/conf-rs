use conf::Conf;

#[derive(Conf, Debug)]
pub struct BadConfig {
    #[conf(parameter, pos, long)]
    pub value: String,
}

fn main() {}
