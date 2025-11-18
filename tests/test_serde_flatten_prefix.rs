#![cfg(feature = "serde")]

use conf::Conf;
use serde_json::json;

/// Child struct that will be flattened with prefix into the parent
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct DatabaseConfig {
    #[arg(long, env)]
    pub host: String,
    #[arg(long, env)]
    pub port: u16,
    #[arg(long, env)]
    pub name: Option<String>,
}

/// Parent struct with a flattened child using prefix
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct AppConfig {
    #[arg(long, env)]
    pub app_name: String,
    #[arg(long, env)]
    pub debug: bool,
    #[conf(flatten, serde(flatten(prefix)))]
    pub database: DatabaseConfig,
}

#[test]
fn test_serde_flatten_prefix_basic() {
    // Fields in JSON have database_ prefix
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=myapp"])
        .env([("DEBUG", "false")])
        .doc(
            "config.json",
            json!({
                "database_host": "localhost",
                "database_port": 5432,
                "database_name": "testdb"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.app_name, "myapp");
    assert!(!result.debug);
    assert_eq!(result.database.host, "localhost");
    assert_eq!(result.database.port, 5432);
    assert_eq!(result.database.name, Some("testdb".to_string()));
}

#[test]
fn test_serde_flatten_prefix_mixed_sources() {
    // Some fields from args (which don't have prefix), some from JSON (which has prefix)
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=myapp", "--host=from_args"])
        .env([("DEBUG", "true"), ("PORT", "3306")])
        .doc(
            "config.json",
            json!({
                "database_name": "from_json"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.app_name, "myapp");
    assert!(result.debug);
    assert_eq!(result.database.host, "from_args");
    assert_eq!(result.database.port, 3306);
    assert_eq!(result.database.name, Some("from_json".to_string()));
}

#[test]
fn test_serde_flatten_prefix_shadowing() {
    // Args shadow JSON values for flattened fields (args don't have prefix)
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=myapp", "--host=from_args", "--port=9999"])
        .env([("DEBUG", "false")])
        .doc(
            "config.json",
            json!({
                "database_host": "from_json",
                "database_port": 5432
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.database.host, "from_args");
    assert_eq!(result.database.port, 9999);
}

#[test]
fn test_serde_flatten_prefix_all_from_json() {
    // All values from JSON with prefix
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=myapp"])
        .env([("DEBUG", "false")])
        .doc(
            "config.json",
            json!({
                "database_host": "jsonhost",
                "database_port": 1234,
                "database_name": "jsondb"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.database.host, "jsonhost");
    assert_eq!(result.database.port, 1234);
    assert_eq!(result.database.name, Some("jsondb".to_string()));
}

/// Test with multiple flattened children with prefixes
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ServerConfig {
    #[arg(long, env)]
    pub host: String,
    #[arg(long, env)]
    pub port: u16,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct MultiConfig {
    #[arg(long, env)]
    pub name: String,
    #[conf(flatten, prefix = "db", serde(flatten(prefix)))]
    pub database: ServerConfig,
    #[conf(flatten, prefix = "cache", serde(flatten(prefix)))]
    pub cache: ServerConfig,
}

#[test]
fn test_serde_flatten_prefix_multiple() {
    // Multiple flattened structs with different prefixes
    let result = MultiConfig::conf_builder()
        .args([".", "--name=multi"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "database_host": "db.example.com",
                "database_port": 5432,
                "cache_host": "cache.example.com",
                "cache_port": 6379
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "multi");
    assert_eq!(result.database.host, "db.example.com");
    assert_eq!(result.database.port, 5432);
    assert_eq!(result.cache.host, "cache.example.com");
    assert_eq!(result.cache.port, 6379);
}

#[test]
fn test_serde_flatten_prefix_partial_override() {
    // Partial CLI override with prefix
    let result = MultiConfig::conf_builder()
        .args([".", "--name=multi", "--db-host=cli_db_host"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "database_host": "json_db_host",
                "database_port": 5432,
                "cache_host": "cache.example.com",
                "cache_port": 6379
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.database.host, "cli_db_host"); // From CLI
    assert_eq!(result.database.port, 5432); // From JSON
    assert_eq!(result.cache.host, "cache.example.com");
    assert_eq!(result.cache.port, 6379);
}

// TODO: Optional flatten with prefix needs work
// The issue is that PrefixStrippingStateMachine's Value type is the inner type's Value,
// but for optional flatten we need Option<T>.

#[test]
fn test_serde_flatten_prefix_unknown_field_rejected() {
    // Unknown fields with the prefix should be rejected
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=myapp"])
        .env([("DEBUG", "false")])
        .doc(
            "config.json",
            json!({
                "database_host": "localhost",
                "database_port": 5432,
                "database_unknown": "should_fail"
            }),
        )
        .try_parse();

    assert!(result.is_err());
}

/// Test with custom prefix string
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct CustomPrefixConfig {
    #[arg(long, env)]
    pub name: String,
    #[conf(flatten, serde(flatten(prefix = "db.")))]
    pub database: ServerConfig,
}

#[test]
fn test_serde_flatten_custom_prefix() {
    // Custom prefix "db." instead of auto-generated "database_"
    let result = CustomPrefixConfig::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "db.host": "customhost",
                "db.port": 9999
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "test");
    assert_eq!(result.database.host, "customhost");
    assert_eq!(result.database.port, 9999);
}

#[test]
fn test_serde_flatten_custom_prefix_cli_override() {
    // CLI args override custom prefixed JSON values
    let result = CustomPrefixConfig::conf_builder()
        .args([".", "--name=test", "--host=from_cli"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "db.host": "from_json",
                "db.port": 8080
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.database.host, "from_cli");
    assert_eq!(result.database.port, 8080);
}

/// Test with empty prefix (fields at root level, like regular flatten)
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct EmptyPrefixConfig {
    #[arg(long, env)]
    pub name: String,
    #[conf(flatten, serde(flatten(prefix = "")))]
    pub server: ServerConfig,
}

#[test]
fn test_serde_flatten_empty_prefix() {
    // Empty prefix means fields are at root level
    let result = EmptyPrefixConfig::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "host": "roothost",
                "port": 7777
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "test");
    assert_eq!(result.server.host, "roothost");
    assert_eq!(result.server.port, 7777);
}
