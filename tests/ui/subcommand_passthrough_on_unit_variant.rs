use conf_derive::Subcommands;

#[derive(Subcommands)]
pub enum Command {
    #[conf(one_of_fields(a, b))]
    Status,
}

fn main() {}
