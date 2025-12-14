//! See completion_example.sh for a test script for this example.
//! This requires the Cargo "completion" feature to be enabled.

use conf::{Conf, Subcommands, completion::Shell};
use http::Uri as Url;
use std::net::SocketAddr;
use std::path::PathBuf;

/// Top-level CLI wrapper
#[derive(Conf, Debug)]
#[conf(name = "completion_example")]
pub struct Cli {
    #[conf(subcommands)]
    pub cmd: CliCommand,
}

#[derive(Subcommands, Debug)]
pub enum CliCommand {
    /// Run the service
    Run(ModelServiceConfig),

    /// Print a shell completion script to stdout
    Completion(CompletionArgs),
}

#[derive(Conf, Debug)]
pub struct CompletionArgs {
    /// Shell to generate completions for (bash|elvish|fish|powershell|zsh)
    #[conf(pos)]
    pub shell: Shell,
}

/// Configuration for an http client
#[derive(Conf, Debug)]
pub struct HttpClientConfig {
    /// Base URL
    #[conf(long, env)]
    pub url: Url,

    /// Number of retries
    #[conf(long, env)]
    pub retries: u32,
}

/// Configuration for model service
#[derive(Conf, Debug)]
pub struct ModelServiceConfig {
    /// Listen address to bind to
    #[conf(long, env, default_value = "127.0.0.1:9090")]
    pub listen_addr: SocketAddr,

    /// Auth service:
    #[conf(flatten, prefix, help_prefix)]
    pub auth: Option<HttpClientConfig>,

    /// Database:
    #[conf(flatten, prefix, help_prefix)]
    pub db: HttpClientConfig,

    /// Optional subcommands
    #[conf(subcommands)]
    pub command: Option<Command>,
}

/// Subcommands that can be used with this service
#[derive(Subcommands, Debug)]
pub enum Command {
    /// Run the migrations
    RunMigrations(MigrationConfig),
    /// Show the pending migrations
    ShowPendingMigrations(MigrationConfig),
}

#[derive(Conf, Debug)]
pub struct MigrationConfig {
    /// Path to migrations file (instead of embedded migrations)
    #[conf(long, env)]
    pub migrations: Option<PathBuf>,
}

fn main() {
    match Cli::parse().cmd {
        CliCommand::Completion(args) => {
            conf::completion::write_completion::<Cli, _>(args.shell, None, &mut std::io::stdout())
                .expect("Expected to output shell script");
        }
        CliCommand::Run(_cfg) => {
            // Your normal execution path goes here
            // (start server, run migrations, etc.)
        }
    }
}
