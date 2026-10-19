use conf::{Conf, Subcommands};

#[derive(Conf)]
struct RunArgs {}

#[derive(Subcommands)]
enum Command {
    // The generated code has a local named `program_options` here, which shouldn't be visible
    #[conf(about = program_options.len().to_string())]
    Run(RunArgs),
}

fn main() {}
