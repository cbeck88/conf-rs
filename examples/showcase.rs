use conf::Conf;
use std::net::SocketAddr;
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

/// Configuration for solver service
#[derive(Conf, Debug)]
pub struct SolveServiceConfig {
    /// Listen address to bind to
    #[conf(long, env, default_value = "127.0.0.1:4040")]
    pub listen_addr: SocketAddr,

    /// Auth service:
    #[conf(flatten, prefix, help_prefix)]
    pub auth: HttpClientConfig,

    /// Artifact service:
    #[conf(flatten, prefix, help_prefix)]
    pub artifact: HttpClientConfig,
}

/// Configuration for solver algorithm
#[derive(Conf, Debug)]
pub struct SolverConfig {
    /// Size of matrix to use when solving
    #[conf(long, env)]
    pub matrix_size: u64,

    /// Branching factor to use when exploring search tree
    #[conf(long, env)]
    pub branching_factor: u32,

    /// Epsilon which controls when we decide that solutions have stopped improving significantly
    #[conf(long, env)]
    pub epsilon: f64,

    /// Whether to use a deterministic seeded rng
    #[conf(long, env)]
    pub deterministic_rng: bool,

    /// Maximum number of rounds before we stop the search
    #[conf(long, env)]
    pub round_limit: Option<u64>,
}

/// Solves widget optimization problems on demand as a service
#[derive(Conf, Debug)]
#[conf(env_prefix = "MYCO_")]
pub struct Config {
    /// Solver service:
    #[conf(flatten, help_prefix)]
    pub solver_service: SolveServiceConfig,

    /// Basic solver:
    #[conf(flatten, prefix, help_prefix)]
    pub basic_solver: SolverConfig,

    /// High-priority solver:
    #[conf(flatten, prefix, help_prefix)]
    pub high_priority_solver: SolverConfig,

    /// Peers to which we can try to loadshed
    #[conf(repeat, long = "peer", env)]
    pub peer_urls: Vec<Url>,

    /// Admin listen address to bind to
    #[conf(long, env, default_value = "127.0.0.1:9090")]
    pub admin_listen_addr: SocketAddr,

    /// Telemetry endpoint:
    #[conf(flatten, prefix, help_prefix)]
    pub telemetry: HttpClientConfig,
}

fn main() {
    let config = Config::parse();

    println!("{config:#?}");
}
