use conf::Conf;
use conf_derive::Subcommands;

#[derive(Conf)]
pub struct RunConfig {
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Subcommands)]
pub enum Command {
    #[conf(one_of_fields(a, b))]
    Run(RunConfig),
}

fn main() {}
