use conf::{Conf, Subcommands};
use http::Uri as Url;
use std::net::SocketAddr;
use std::path::PathBuf;

/// Configuration for an http client
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct HttpClientConfig {
    /// Base URL
    #[arg(long, env, serde(use_value_parser))]
    pub url: Url,

    /// Number of retries
    #[arg(long, env)]
    pub retries: u32,
}

/// Configuration for model service
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ModelServiceConfig {
    /// Listen address to bind to
    #[arg(long, env, default_value = "127.0.0.1:9090")]
    pub listen_addr: SocketAddr,

    /// Auth service:
    #[arg(flatten, prefix, help_prefix)]
    pub auth: Option<HttpClientConfig>,

    /// Database:
    #[arg(flatten, prefix, help_prefix)]
    pub db: HttpClientConfig,

    /// Config file path
    #[arg(long = "config", env = "CONFIG")]
    pub config_file: Option<PathBuf>,

    /// Optional subcommands
    #[arg(subcommands)]
    pub command: Option<Command>,
}

/// Subcommands that can be used with this service
#[derive(Subcommands, Debug)]
#[conf(serde)]
pub enum Command {
    #[conf(serde(rename = "migrations"))]
    RunMigrations(MigrationConfig),
    #[conf(serde(rename = "migrations"))]
    ShowPendingMigrations(MigrationConfig),
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct MigrationConfig {
    /// Path to migrations file (instead of embedded migrations)
    #[arg(long, env)]
    pub sql_file: Option<PathBuf>,
}
