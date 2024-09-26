use assert_matches::assert_matches;
use conf::{Conf, ParseType};

mod common;
use common::*;

#[derive(Conf, Debug)]
struct HttpClientConfig {
    /// Url
    #[conf(long, env)]
    url: String,

    /// Number of retries
    #[conf(long, env, default_value = "3")]
    retries: u32,
}

// a putative test client that flattens http client config
#[derive(Conf, Debug)]
struct TestClientConfig {
    /// api endpoint to test
    #[conf(flatten)]
    http: HttpClientConfig,

    /// debug output
    #[conf(long)]
    debug: bool,
}

#[test]
fn test_client_config_program_options() {
    let parser_config = TestClientConfig::get_parser_config().unwrap();
    let opts = TestClientConfig::get_program_options().unwrap();

    assert!(!parser_config.no_help_flag);
    assert!(parser_config.about.is_none());

    assert_eq!(opts.len(), 3);

    assert_eq!(opts[0].parse_type, ParseType::Parameter);
    assert_eq!(opts[0].short_form, None);
    assert_eq!(opts[0].long_form.as_deref(), Some("url"));
    assert_eq!(opts[0].env_form.as_deref(), Some("URL"));
    assert_eq!(opts[0].default_value.as_deref(), None);
    assert!(opts[0].is_required);
    assert_eq!(opts[0].description.as_deref(), Some("Url"));

    assert_eq!(opts[1].parse_type, ParseType::Parameter);
    assert_eq!(opts[1].short_form, None);
    assert_eq!(opts[1].long_form.as_deref(), Some("retries"));
    assert_eq!(opts[1].env_form.as_deref(), Some("RETRIES"));
    assert_eq!(opts[1].default_value.as_deref(), Some("3"));
    assert!(!opts[1].is_required);
    assert_eq!(opts[1].description.as_deref(), Some("Number of retries"));

    assert_eq!(opts[2].parse_type, ParseType::Flag);
    assert_eq!(opts[2].short_form, None);
    assert_eq!(opts[2].long_form.as_deref(), Some("debug"));
    assert_eq!(opts[2].env_form.as_deref(), None);
    assert_eq!(opts[2].default_value, None);
    assert!(!opts[2].is_required);
    assert_eq!(opts[2].description.as_deref(), Some("debug output"));
}

#[test]
fn test_client_config_parsing() {
    let result = TestClientConfig::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("URL", "http://example.com")],
    )
    .unwrap();

    assert_eq!(result.http.url, "http://example.com");
    assert_eq!(result.http.retries, 3);
    assert!(!result.debug);

    let result = TestClientConfig::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![("URL", "http://example.com"), ("RETRIES", "20")],
    )
    .unwrap();

    assert_eq!(result.http.url, "http://example.com");
    assert_eq!(result.http.retries, 20);
    assert!(!result.debug);

    let result = TestClientConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "--debug"],
        vec![("URL", "http://example.com"), ("RETRIES", "20")],
    )
    .unwrap();

    assert_eq!(result.http.url, "http://example.com");
    assert_eq!(result.http.retries, 20);
    assert!(result.debug);

    let result = TestClientConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "--debug", "--retries=99"],
        vec![("URL", "http://example.com"), ("RETRIES", "20")],
    )
    .unwrap();

    assert_eq!(result.http.url, "http://example.com");
    assert_eq!(result.http.retries, 99);
    assert!(result.debug);
}

#[derive(Conf, Debug)]
struct DbConfig {
    /// Url
    #[conf(long, env)]
    url: String,

    /// Password
    #[conf(long, env)]
    password: String,

    /// Connection pool size
    #[conf(long, env, default_value = "1")]
    connection_pool_size: usize,
}

// a putative tool that connects to a database and flattens db config, with prefixing enabled
#[derive(Conf, Debug)]
struct TestDbToolConfig {
    /// Database
    #[conf(flatten, prefix)]
    db: DbConfig,

    /// If set, don't make changes to the database
    #[conf(short, long)]
    dry_run: bool,
}

#[test]
fn test_db_tool_config_program_options() {
    let parser_config = TestDbToolConfig::get_parser_config().unwrap();
    let opts = TestDbToolConfig::get_program_options().unwrap();

    assert!(!parser_config.no_help_flag);
    assert!(parser_config.about.is_none());

    assert_eq!(opts.len(), 4);

    assert_eq!(opts[0].parse_type, ParseType::Parameter);
    assert_eq!(opts[0].short_form, None);
    assert_eq!(opts[0].long_form.as_deref(), Some("db-url"));
    assert_eq!(opts[0].env_form.as_deref(), Some("DB_URL"));
    assert_eq!(opts[0].default_value.as_deref(), None);
    assert!(opts[0].is_required);
    assert_eq!(opts[0].description.as_deref(), Some("Url")); // help prefixing was not enabled

    assert_eq!(opts[1].parse_type, ParseType::Parameter);
    assert_eq!(opts[1].short_form, None);
    assert_eq!(opts[1].long_form.as_deref(), Some("db-password"));
    assert_eq!(opts[1].env_form.as_deref(), Some("DB_PASSWORD"));
    assert_eq!(opts[1].default_value.as_deref(), None);
    assert!(opts[1].is_required);
    assert_eq!(opts[1].description.as_deref(), Some("Password"));

    assert_eq!(opts[2].parse_type, ParseType::Parameter);
    assert_eq!(opts[2].short_form, None);
    assert_eq!(
        opts[2].long_form.as_deref(),
        Some("db-connection-pool-size")
    );
    assert_eq!(opts[2].env_form.as_deref(), Some("DB_CONNECTION_POOL_SIZE"));
    assert_eq!(opts[2].default_value.as_deref(), Some("1"));
    assert!(!opts[2].is_required);
    assert_eq!(opts[2].description.as_deref(), Some("Connection pool size"));

    assert_eq!(opts[3].parse_type, ParseType::Flag);
    assert_eq!(opts[3].short_form, Some('d'));
    assert_eq!(opts[3].long_form.as_deref(), Some("dry-run"));
    assert_eq!(opts[3].env_form.as_deref(), None);
    assert_eq!(opts[3].default_value, None);
    assert!(!opts[3].is_required);
    assert_eq!(
        opts[3].description.as_deref(),
        Some("If set, don't make changes to the database")
    );
}

#[test]
fn test_db_tool_config_parsing() {
    let result = TestDbToolConfig::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![
            ("DB_URL", "postgres://dev@example.local"),
            ("DB_PASSWORD", "hunter42"),
        ],
    )
    .unwrap();

    assert_eq!(result.db.url, "postgres://dev@example.local");
    assert_eq!(result.db.password, "hunter42");
    assert_eq!(result.db.connection_pool_size, 1);
    assert!(!result.dry_run);

    let result = TestDbToolConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "--dry-run"],
        vec![
            ("DB_URL", "postgres://dev@example.local"),
            ("DB_PASSWORD", "hunter42"),
        ],
    )
    .unwrap();

    assert_eq!(result.db.url, "postgres://dev@example.local");
    assert_eq!(result.db.password, "hunter42");
    assert_eq!(result.db.connection_pool_size, 1);
    assert!(result.dry_run);

    // DRY_RUN env is ignored because there is no env form specified for that option
    let result = TestDbToolConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "--db-connection-pool-size=6"],
        vec![
            ("DB_URL", "postgres://dev@example.local"),
            ("DB_PASSWORD", "hunter42"),
            ("DRY_RUN", ""),
        ],
    )
    .unwrap();

    assert_eq!(result.db.url, "postgres://dev@example.local");
    assert_eq!(result.db.password, "hunter42");
    assert_eq!(result.db.connection_pool_size, 6);
    assert!(!result.dry_run);
}

// a putatative test service, which flattens HttpClientConfig multiple times, as well as DbConfig
#[derive(Conf, Debug)]
struct TestServiceConfig {
    /// Listen addr to bind to
    #[conf(long, env, default_value = "127.0.0.1:4040")]
    listen_addr: std::net::SocketAddr,

    /// Database
    #[conf(flatten, prefix, help_prefix)]
    db: DbConfig,

    /// Auth Service
    #[conf(flatten, prefix, help_prefix)]
    auth_service: HttpClientConfig,

    /// Friend Service
    #[conf(flatten, prefix, help_prefix)]
    friend_service: HttpClientConfig,

    /// Buddy Service
    #[conf(flatten, prefix, help_prefix)]
    buddy_service: HttpClientConfig,

    /// This is a very important option which must be used very, very carefully.
    /// I literally could write an entire book about this option and it's proper use.
    /// In fact I'm essentially doing that right now.
    ///
    /// I really hope to god that someone actually reads all this and that by
    /// the time all this text is displayed to the operator via the CLI --help,
    /// it has not been hopelessly mangled.
    #[conf(long)]
    hard_mode: bool,
}

#[test]
fn test_service_config_program_options() {
    let parser_config = TestServiceConfig::get_parser_config().unwrap();
    let opts = TestServiceConfig::get_program_options().unwrap();

    assert!(!parser_config.no_help_flag);
    assert!(parser_config.about.is_none());

    let mut iter = opts.iter();

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("listen-addr"));
    assert_eq!(opt.env_form.as_deref(), Some("LISTEN_ADDR"));
    assert_eq!(opt.default_value.as_deref(), Some("127.0.0.1:4040"));
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Listen addr to bind to"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("db-url"));
    assert_eq!(opt.env_form.as_deref(), Some("DB_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Database Url")); // help prefixing was enabled

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("db-password"));
    assert_eq!(opt.env_form.as_deref(), Some("DB_PASSWORD"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Database Password"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("db-connection-pool-size"));
    assert_eq!(opt.env_form.as_deref(), Some("DB_CONNECTION_POOL_SIZE"));
    assert_eq!(opt.default_value.as_deref(), Some("1"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Database Connection pool size")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("auth-service-url"));
    assert_eq!(opt.env_form.as_deref(), Some("AUTH_SERVICE_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Auth Service Url")); // help prefixing was enabled

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("auth-service-retries"));
    assert_eq!(opt.env_form.as_deref(), Some("AUTH_SERVICE_RETRIES"));
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Auth Service Number of retries")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("friend-service-url"));
    assert_eq!(opt.env_form.as_deref(), Some("FRIEND_SERVICE_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Friend Service Url"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("friend-service-retries"));
    assert_eq!(opt.env_form.as_deref(), Some("FRIEND_SERVICE_RETRIES"));
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Friend Service Number of retries")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("buddy-service-url"));
    assert_eq!(opt.env_form.as_deref(), Some("BUDDY_SERVICE_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Buddy Service Url"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("buddy-service-retries"));
    assert_eq!(opt.env_form.as_deref(), Some("BUDDY_SERVICE_RETRIES"));
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Buddy Service Number of retries")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Flag);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("hard-mode"));
    assert_eq!(opt.env_form.as_deref(), None);
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("This is a very important option which must be used very, very carefully.\nI literally could write an entire book about this option and it's proper use.\nIn fact I'm essentially doing that right now.\n\nI really hope to god that someone actually reads all this and that by\nthe time all this text is displayed to the operator via the CLI --help,\nit has not been hopelessly mangled."));

    assert_matches!(iter.next(), None);
}

#[test]
fn test_service_config_parsing() {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    assert_error_contains_text!(
        TestServiceConfig::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        [
            "required value was not provided",
            "env 'DB_URL'",
            "env 'DB_PASSWORD'",
            "env 'AUTH_SERVICE_URL'",
            "env 'FRIEND_SERVICE_URL'",
            "env 'BUDDY_SERVICE_URL'"
        ]
    );

    let result = TestServiceConfig::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![
            ("DB_URL", "postgres://db.local"),
            ("DB_PASSWORD", "hunter42"),
            ("AUTH_SERVICE_URL", "http://auth.service.local"),
            ("FRIEND_SERVICE_URL", "http://friend.service.local"),
            ("BUDDY_SERVICE_URL", "http://buddy.service.local"),
        ],
    )
    .unwrap();

    assert_eq!(
        result.listen_addr,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4040)
    );
    assert_eq!(result.db.url, "postgres://db.local");
    assert_eq!(result.db.password, "hunter42");
    assert_eq!(result.db.connection_pool_size, 1);
    assert_eq!(result.auth_service.url, "http://auth.service.local");
    assert_eq!(result.auth_service.retries, 3);
    assert_eq!(result.friend_service.url, "http://friend.service.local");
    assert_eq!(result.friend_service.retries, 3);
    assert_eq!(result.buddy_service.url, "http://buddy.service.local");
    assert_eq!(result.buddy_service.retries, 3);
    assert!(!result.hard_mode);

    let result = TestServiceConfig::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--friend-service-retries=5",
            "--hard-mode",
            "--db-connection-pool-size",
            "20",
        ],
        vec![
            ("DB_URL", "postgres://db.local"),
            ("DB_PASSWORD", "hunter42"),
            ("AUTH_SERVICE_URL", "http://auth.service.local"),
            ("FRIEND_SERVICE_URL", "http://friend.service.local"),
            ("BUDDY_SERVICE_URL", "http://buddy.service.local"),
        ],
    )
    .unwrap();

    assert_eq!(
        result.listen_addr,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4040)
    );
    assert_eq!(result.db.url, "postgres://db.local");
    assert_eq!(result.db.password, "hunter42");
    assert_eq!(result.db.connection_pool_size, 20);
    assert_eq!(result.auth_service.url, "http://auth.service.local");
    assert_eq!(result.auth_service.retries, 3);
    assert_eq!(result.friend_service.url, "http://friend.service.local");
    assert_eq!(result.friend_service.retries, 5);
    assert_eq!(result.buddy_service.url, "http://buddy.service.local");
    assert_eq!(result.buddy_service.retries, 3);
    assert!(result.hard_mode);
}

#[derive(Conf, Debug)]
struct PeerServiceConfig {
    /// Urls to connect to
    #[conf(repeat, long = "url", env)]
    urls: Vec<String>,

    /// Minimum number of connected peers required for readiness / safe operation
    #[conf(long, env, default_value = "2")]
    min_connections: u32,

    /// If a peer's badness score exceeds this limit, the peer is automatically disconnected
    /// This will lead to an alert and we will not reconnect to the peer until an administrator
    /// authorizes blah blah blah
    #[conf(long, env, default_value = "1000")]
    badness_score_limit: u32,
}

// A putative server binary that embeds test-service as well as a peering service and an admin
// endpoint This tests what happened if we have two layers of flattening, and also if we flatten
// some structs at layer 2 and also again at layer 1, and turning prefixing on and off for different
// structs. This also tests top-level env prefix
#[derive(Conf, Debug)]
#[conf(env_prefix = "FROB_")]
struct FrobConfig {
    /// Test
    #[conf(flatten)]
    test: TestServiceConfig,

    /// Peering:
    #[conf(flatten, prefix, help_prefix)]
    peer: PeerServiceConfig,

    /// Listen addr to bind admin endpoint to
    #[conf(long, env, default_value = "127.0.0.1:9090")]
    admin_listen_addr: std::net::SocketAddr,

    /// Slack webhook config
    #[conf(flatten, prefix, help_prefix = "Slack")]
    slack: HttpClientConfig,
}

#[test]
fn frob_config_program_options() {
    let parser_config = FrobConfig::get_parser_config().unwrap();
    let opts = FrobConfig::get_program_options().unwrap();

    assert!(!parser_config.no_help_flag);
    assert!(parser_config.about.is_none());

    let mut iter = opts.iter();

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("listen-addr"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_LISTEN_ADDR"));
    assert_eq!(opt.default_value.as_deref(), Some("127.0.0.1:4040"));
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Listen addr to bind to"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("db-url"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_DB_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Database Url")); // help prefixing was enabled

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("db-password"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_DB_PASSWORD"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Database Password"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("db-connection-pool-size"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_DB_CONNECTION_POOL_SIZE")
    );
    assert_eq!(opt.default_value.as_deref(), Some("1"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Database Connection pool size")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("auth-service-url"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_AUTH_SERVICE_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Auth Service Url")); // help prefixing was enabled

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("auth-service-retries"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_AUTH_SERVICE_RETRIES"));
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Auth Service Number of retries")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("friend-service-url"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_FRIEND_SERVICE_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Friend Service Url"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("friend-service-retries"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_FRIEND_SERVICE_RETRIES"));
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Friend Service Number of retries")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("buddy-service-url"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_BUDDY_SERVICE_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Buddy Service Url"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("buddy-service-retries"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_BUDDY_SERVICE_RETRIES"));
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Buddy Service Number of retries")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Flag);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("hard-mode"));
    assert_eq!(opt.env_form.as_deref(), None);
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("This is a very important option which must be used very, very carefully.\nI literally could write an entire book about this option and it's proper use.\nIn fact I'm essentially doing that right now.\n\nI really hope to god that someone actually reads all this and that by\nthe time all this text is displayed to the operator via the CLI --help,\nit has not been hopelessly mangled."));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Repeat);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("peer-url"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_PEER_URLS"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Peering: Urls to connect to")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("peer-min-connections"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_PEER_MIN_CONNECTIONS"));
    assert_eq!(opt.default_value.as_deref(), Some("2"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Peering: Minimum number of connected peers required for readiness / safe operation")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("peer-badness-score-limit"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_PEER_BADNESS_SCORE_LIMIT")
    );
    assert_eq!(opt.default_value.as_deref(), Some("1000"));
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Peering:\nIf a peer's badness score exceeds this limit, the peer is automatically disconnected\nThis will lead to an alert and we will not reconnect to the peer until an administrator\nauthorizes blah blah blah"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("admin-listen-addr"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_ADMIN_LISTEN_ADDR"));
    assert_eq!(opt.default_value.as_deref(), Some("127.0.0.1:9090"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Listen addr to bind admin endpoint to")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("slack-url"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_SLACK_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Slack Url"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("slack-retries"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_SLACK_RETRIES"));
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Slack Number of retries"));

    assert_matches!(iter.next(), None);
}

#[test]
fn frob_config_parsing() {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    assert_error_contains_text!(
        FrobConfig::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        [
            "required value was not provided",
            "env 'FROB_DB_URL'",
            "env 'FROB_DB_PASSWORD'",
            "env 'FROB_AUTH_SERVICE_URL'",
            "env 'FROB_FRIEND_SERVICE_URL'",
            "env 'FROB_BUDDY_SERVICE_URL'",
            "env 'FROB_SLACK_URL'"
        ],
        not["env 'FROB_PEER_URLS'"]
    );

    let result = FrobConfig::try_parse_from::<&str, &str, &str>(
        vec!["."],
        vec![
            ("FROB_DB_URL", "postgres://db.local"),
            ("FROB_DB_PASSWORD", "hunter42"),
            ("FROB_AUTH_SERVICE_URL", "http://auth.service.local"),
            ("FROB_FRIEND_SERVICE_URL", "http://friend.service.local"),
            ("FROB_BUDDY_SERVICE_URL", "http://buddy.service.local"),
            ("FROB_SLACK_URL", "https://slack.com/asdf/jkl"),
            ("FROB_PEER_URLS", "a.com,b.com,c.com"),
        ],
    )
    .unwrap();

    assert_eq!(
        result.test.listen_addr,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4040)
    );
    assert_eq!(result.test.db.url, "postgres://db.local");
    assert_eq!(result.test.db.password, "hunter42");
    assert_eq!(result.test.db.connection_pool_size, 1);
    assert_eq!(result.test.auth_service.url, "http://auth.service.local");
    assert_eq!(result.test.auth_service.retries, 3);
    assert_eq!(
        result.test.friend_service.url,
        "http://friend.service.local"
    );
    assert_eq!(result.test.friend_service.retries, 3);
    assert_eq!(result.test.buddy_service.url, "http://buddy.service.local");
    assert_eq!(result.test.buddy_service.retries, 3);
    assert!(!result.test.hard_mode);
    assert_eq!(result.peer.urls, vec_str(["a.com", "b.com", "c.com"]));
    assert_eq!(result.peer.min_connections, 2);
    assert_eq!(result.peer.badness_score_limit, 1000);
    assert_eq!(
        result.admin_listen_addr,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9090)
    );
    assert_eq!(result.slack.url, "https://slack.com/asdf/jkl");
    assert_eq!(result.slack.retries, 3);

    let result = FrobConfig::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "--peer-url",
            "a.net",
            "--peer-url",
            "b.org",
            "--peer-url=c.io",
        ],
        vec![
            ("FROB_DB_URL", "postgres://db.local"),
            ("FROB_DB_PASSWORD", "hunter42"),
            ("FROB_AUTH_SERVICE_URL", "http://auth.service.local"),
            ("FROB_FRIEND_SERVICE_URL", "http://friend.service.local"),
            ("FROB_BUDDY_SERVICE_URL", "http://buddy.service.local"),
            ("FROB_SLACK_URL", "https://slack.com/asdf/jkl"),
            ("FROB_PEER_URLS", "a.com,b.com,c.com"),
        ],
    )
    .unwrap();

    assert_eq!(
        result.test.listen_addr,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4040)
    );
    assert_eq!(result.test.db.url, "postgres://db.local");
    assert_eq!(result.test.db.password, "hunter42");
    assert_eq!(result.test.db.connection_pool_size, 1);
    assert_eq!(result.test.auth_service.url, "http://auth.service.local");
    assert_eq!(result.test.auth_service.retries, 3);
    assert_eq!(
        result.test.friend_service.url,
        "http://friend.service.local"
    );
    assert_eq!(result.test.friend_service.retries, 3);
    assert_eq!(result.test.buddy_service.url, "http://buddy.service.local");
    assert_eq!(result.test.buddy_service.retries, 3);
    assert!(!result.test.hard_mode);
    assert_eq!(result.peer.urls, vec_str(["a.net", "b.org", "c.io"]));
    assert_eq!(result.peer.min_connections, 2);
    assert_eq!(result.peer.badness_score_limit, 1000);
    assert_eq!(
        result.admin_listen_addr,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9090)
    );
    assert_eq!(result.slack.url, "https://slack.com/asdf/jkl");
    assert_eq!(result.slack.retries, 3);
}

// Another version of FrobConfig that uses different flattening options
#[allow(dead_code)]
#[derive(Conf)]
#[conf(env_prefix = "FROB_")]
struct FrobConfig2 {
    /// Test
    #[conf(flatten, long_prefix = "t-", env_prefix = "TEST_SERVICE_")]
    test: TestServiceConfig,

    /// Peering:
    #[conf(flatten, env_prefix = "PEER_", long_prefix = "peers-", help_prefix)]
    peer: PeerServiceConfig,

    /// Listen addr to bind admin endpoint to
    #[conf(long, env, default_value = "127.0.0.1:9090")]
    admin_listen_addr: std::net::SocketAddr,

    /// Slack webhook config
    #[conf(flatten, prefix, help_prefix = "Slack")]
    slack: HttpClientConfig,
}

#[test]
fn frob_config2_program_options() {
    let parser_config = FrobConfig2::get_parser_config().unwrap();
    let opts = FrobConfig2::get_program_options().unwrap();

    assert!(!parser_config.no_help_flag);
    assert!(parser_config.about.is_none());

    let mut iter = opts.iter();

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-listen-addr"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_TEST_SERVICE_LISTEN_ADDR")
    );
    assert_eq!(opt.default_value.as_deref(), Some("127.0.0.1:4040"));
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Listen addr to bind to"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-db-url"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_TEST_SERVICE_DB_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Database Url")); // help prefixing was enabled

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-db-password"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_TEST_SERVICE_DB_PASSWORD")
    );
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Database Password"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-db-connection-pool-size"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_TEST_SERVICE_DB_CONNECTION_POOL_SIZE")
    );
    assert_eq!(opt.default_value.as_deref(), Some("1"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Database Connection pool size")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-auth-service-url"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_TEST_SERVICE_AUTH_SERVICE_URL")
    );
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Auth Service Url")); // help prefixing was enabled

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-auth-service-retries"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_TEST_SERVICE_AUTH_SERVICE_RETRIES")
    );
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Auth Service Number of retries")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-friend-service-url"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_TEST_SERVICE_FRIEND_SERVICE_URL")
    );
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Friend Service Url"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-friend-service-retries"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_TEST_SERVICE_FRIEND_SERVICE_RETRIES")
    );
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Friend Service Number of retries")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-buddy-service-url"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_TEST_SERVICE_BUDDY_SERVICE_URL")
    );
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Buddy Service Url"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-buddy-service-retries"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_TEST_SERVICE_BUDDY_SERVICE_RETRIES")
    );
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Buddy Service Number of retries")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Flag);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("t-hard-mode"));
    assert_eq!(opt.env_form.as_deref(), None);
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("This is a very important option which must be used very, very carefully.\nI literally could write an entire book about this option and it's proper use.\nIn fact I'm essentially doing that right now.\n\nI really hope to god that someone actually reads all this and that by\nthe time all this text is displayed to the operator via the CLI --help,\nit has not been hopelessly mangled."));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Repeat);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("peers-url"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_PEER_URLS"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Peering: Urls to connect to")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("peers-min-connections"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_PEER_MIN_CONNECTIONS"));
    assert_eq!(opt.default_value.as_deref(), Some("2"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Peering: Minimum number of connected peers required for readiness / safe operation")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("peers-badness-score-limit"));
    assert_eq!(
        opt.env_form.as_deref(),
        Some("FROB_PEER_BADNESS_SCORE_LIMIT")
    );
    assert_eq!(opt.default_value.as_deref(), Some("1000"));
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Peering:\nIf a peer's badness score exceeds this limit, the peer is automatically disconnected\nThis will lead to an alert and we will not reconnect to the peer until an administrator\nauthorizes blah blah blah"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("admin-listen-addr"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_ADMIN_LISTEN_ADDR"));
    assert_eq!(opt.default_value.as_deref(), Some("127.0.0.1:9090"));
    assert!(!opt.is_required);
    assert_eq!(
        opt.description.as_deref(),
        Some("Listen addr to bind admin endpoint to")
    );

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("slack-url"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_SLACK_URL"));
    assert_eq!(opt.default_value.as_deref(), None);
    assert!(opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Slack Url"));

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.short_form, None);
    assert_eq!(opt.long_form.as_deref(), Some("slack-retries"));
    assert_eq!(opt.env_form.as_deref(), Some("FROB_SLACK_RETRIES"));
    assert_eq!(opt.default_value.as_deref(), Some("3"));
    assert!(!opt.is_required);
    assert_eq!(opt.description.as_deref(), Some("Slack Number of retries"));

    assert_matches!(iter.next(), None);
}
