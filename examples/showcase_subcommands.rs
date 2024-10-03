use conf::{Conf, Subcommands};
use std::net::SocketAddr;
use std::path::PathBuf;
use url::Url;

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
    RunMigrations(MigrationConfig),
    ShowPendingMigrations(MigrationConfig),
}

#[derive(Conf, Debug)]
pub struct MigrationConfig {
    /// Path to migrations file (instead of embedded migrations)
    #[conf(long, env)]
    pub migrations: Option<PathBuf>,
}

fn main() {
    let config = ModelServiceConfig::parse();

    println!("{config:#?}");
}
