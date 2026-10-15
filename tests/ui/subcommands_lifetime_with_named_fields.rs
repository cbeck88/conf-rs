use conf::{Conf, Subcommands};

#[derive(Conf)]
pub struct Args {
    #[conf(long)]
    pub x: u32,
}

#[derive(Subcommands)]
pub enum Command<'a> {
    Run(Args),
    Stop {
        #[conf(long)]
        force: bool,
    },
}

fn main() {}
