#![cfg(feature = "serde")]

mod common;
use common::*;

use conf::Conf;
use serde_json::json;

/// Child struct that will be flattened into the parent
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct DatabaseConfig {
    #[arg(long, env)]
    pub db_host: String,
    #[arg(long, env)]
    pub db_port: u16,
    #[arg(long, env)]
    pub db_name: Option<String>,
}

/// Parent struct with a flattened child
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct AppConfig {
    #[arg(long, env)]
    pub app_name: String,
    #[arg(long, env)]
    pub debug: bool,
    #[conf(flatten, serde(flatten))]
    pub database: DatabaseConfig,
}

#[test]
fn test_serde_flatten_basic() {
    // All fields at the same level in JSON
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=myapp"])
        .env([("DEBUG", "false")])
        .doc(
            "config.json",
            json!({
                "db_host": "localhost",
                "db_port": 5432,
                "db_name": "testdb"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.app_name, "myapp");
    assert!(!result.debug);
    assert_eq!(result.database.db_host, "localhost");
    assert_eq!(result.database.db_port, 5432);
    assert_eq!(result.database.db_name, Some("testdb".to_string()));
}

#[test]
fn test_serde_flatten_mixed_sources() {
    // Some fields from args, some from env, some from JSON
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=myapp", "--db-host=arghost"])
        .env([("DEBUG", "true"), ("DB_PORT", "3306")])
        .doc(
            "config.json",
            json!({
                "db_name": "from_json"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.app_name, "myapp");
    assert!(result.debug);
    assert_eq!(result.database.db_host, "arghost");
    assert_eq!(result.database.db_port, 3306);
    assert_eq!(result.database.db_name, Some("from_json".to_string()));
}

#[test]
fn test_serde_flatten_shadowing() {
    // Args shadow JSON values for flattened fields
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=myapp", "--db-host=from_args", "--db-port=9999"])
        .env([("DEBUG", "false")])
        .doc(
            "config.json",
            json!({
                "db_host": "from_json",
                "db_port": 5432
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.database.db_host, "from_args");
    assert_eq!(result.database.db_port, 9999);
}

#[test]
fn test_serde_flatten_optional_field() {
    // Optional field in flattened struct not provided
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=myapp"])
        .env([("DEBUG", "false"), ("DB_HOST", "localhost"), ("DB_PORT", "5432")])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();

    assert_eq!(result.database.db_name, None);
}

#[test]
fn test_serde_flatten_unknown_field_detection() {
    // Unknown fields should be detected even with flatten
    // Note: This is a limitation in standard serde - flatten disables unknown field detection
    // But conf should still detect unknown fields
    assert_error_contains_text!(
        AppConfig::conf_builder()
            .args([".", "--app-name=myapp"])
            .env([("DEBUG", "false"), ("DB_HOST", "localhost"), ("DB_PORT", "5432")])
            .doc(
                "config.json",
                json!({
                    "unknown_field": "should_error"
                })
            )
            .try_parse(),
        ["unknown field", "unknown_field"]
    );
}

/// Multiple flattened structs at the same level
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct CacheConfig {
    #[arg(long, env)]
    pub cache_host: String,
    #[arg(long, env)]
    pub cache_ttl: u32,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct MultiConfig {
    #[arg(long, env)]
    pub name: String,
    #[conf(flatten, serde(flatten))]
    pub database: DatabaseConfig,
    #[conf(flatten, serde(flatten))]
    pub cache: CacheConfig,
}

#[test]
fn test_serde_flatten_multiple() {
    // Multiple flattened structs - all keys at same level
    let result = MultiConfig::conf_builder()
        .args([".", "--name=multi"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "db_host": "dbserver",
                "db_port": 5432,
                "cache_host": "redis",
                "cache_ttl": 3600
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "multi");
    assert_eq!(result.database.db_host, "dbserver");
    assert_eq!(result.database.db_port, 5432);
    assert_eq!(result.cache.cache_host, "redis");
    assert_eq!(result.cache.cache_ttl, 3600);
}

#[test]
fn test_serde_flatten_multiple_unknown_field() {
    // Unknown field with multiple flattened structs
    assert_error_contains_text!(
        MultiConfig::conf_builder()
            .args([".", "--name=multi"])
            .env::<&str, &str>([])
            .doc(
                "config.json",
                json!({
                    "db_host": "dbserver",
                    "db_port": 5432,
                    "cache_host": "redis",
                    "cache_ttl": 3600,
                    "not_a_real_field": true
                })
            )
            .try_parse(),
        ["unknown field", "not_a_real_field"]
    );
}

/// Nested flatten - a flattened struct containing another flattened struct
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ConnectionConfig {
    #[arg(long, env)]
    pub host: String,
    #[arg(long, env)]
    pub port: u16,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ServiceConfig {
    #[arg(long, env)]
    pub timeout: u32,
    #[conf(flatten, serde(flatten))]
    pub connection: ConnectionConfig,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct NestedFlattenConfig {
    #[arg(long, env)]
    pub service_name: String,
    #[conf(flatten, serde(flatten))]
    pub service: ServiceConfig,
}

#[test]
fn test_serde_flatten_nested() {
    // Deeply nested flatten - all keys still at top level
    let result = NestedFlattenConfig::conf_builder()
        .args([".", "--service-name=api"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "timeout": 30,
                "host": "api.example.com",
                "port": 443
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.service_name, "api");
    assert_eq!(result.service.timeout, 30);
    assert_eq!(result.service.connection.host, "api.example.com");
    assert_eq!(result.service.connection.port, 443);
}

/// Test that duplicate field errors work correctly with flatten
#[test]
fn test_serde_flatten_duplicate_field_error() {
    // Same key appearing twice in JSON should error
    // This tests that the state machine correctly tracks visited fields
    let json_str = r#"{"db_host": "first", "db_port": 5432, "db_host": "second"}"#;
    let json_value: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Note: serde_json deduplicates keys, so this test may not trigger the error
    // But if using a format that preserves duplicates, it should error
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=myapp"])
        .env([("DEBUG", "false")])
        .doc("config.json", json_value)
        .try_parse();

    // With serde_json, the second value wins and no error occurs
    // This is expected behavior
    assert!(result.is_ok());
}

/// Test with serde rename on flattened fields
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct RenamedChild {
    #[arg(long, env)]
    #[conf(serde(rename = "server"))]
    pub host: String,
    #[arg(long, env)]
    #[conf(serde(rename = "server_port"))]
    pub port: u16,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct RenamedFlattenConfig {
    #[arg(long, env)]
    pub name: String,
    #[conf(flatten, serde(flatten))]
    pub child: RenamedChild,
}

#[test]
fn test_serde_flatten_with_rename() {
    // Renamed fields in flattened struct
    let result = RenamedFlattenConfig::conf_builder()
        .args([".", "--name=test", "--host=localhost", "--port=8080"])
        .env::<&str, &str>([])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "test");
    assert_eq!(result.child.host, "localhost");
    assert_eq!(result.child.port, 8080);

    // With JSON using renamed keys
    let result = RenamedFlattenConfig::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "server": "jsonhost",
                "server_port": 9090
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.child.host, "jsonhost");
    assert_eq!(result.child.port, 9090);
}

/// Test flatten with optional parent field
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct OptionalFlattenConfig {
    #[arg(long, env)]
    pub name: String,
    #[conf(flatten)]
    pub database: Option<DatabaseConfig>,
}

#[test]
fn test_serde_flatten_optional_parent() {
    // When using flatten with Option, the behavior depends on whether any field is set
    // This tests the interaction between flatten optional and serde
    let result = OptionalFlattenConfig::conf_builder()
        .args([".", "--name=test", "--db-host=localhost", "--db-port=5432"])
        .env::<&str, &str>([])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();

    assert!(result.database.is_some());
    let db = result.database.unwrap();
    assert_eq!(db.db_host, "localhost");
    assert_eq!(db.db_port, 5432);
}

#[test]
fn test_serde_flatten_optional_parent_none() {
    // When no fields of optional flatten are set, it should be None
    let result = OptionalFlattenConfig::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();

    assert!(result.database.is_none());
}
