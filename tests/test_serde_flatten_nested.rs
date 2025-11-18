#![cfg(feature = "serde")]

mod common;
use common::*;

use conf::Conf;
use serde_json::json;

/// Three levels deep: App -> Service -> Connection
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
    pub timeout_ms: u32,
    #[arg(long, env)]
    pub retries: Option<u8>,
    #[conf(flatten, serde(flatten))]
    pub connection: ConnectionConfig,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct AppConfig {
    #[arg(long, env)]
    pub app_name: String,
    #[arg(long, env)]
    pub debug: bool,
    #[conf(flatten, serde(flatten))]
    pub service: ServiceConfig,
}

#[test]
fn test_three_levels_all_from_json() {
    let result = AppConfig::conf_builder()
        .args(["."])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "app_name": "myapp",
                "debug": true,
                "timeout_ms": 5000,
                "retries": 3,
                "host": "localhost",
                "port": 8080
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.app_name, "myapp");
    assert!(result.debug);
    assert_eq!(result.service.timeout_ms, 5000);
    assert_eq!(result.service.retries, Some(3));
    assert_eq!(result.service.connection.host, "localhost");
    assert_eq!(result.service.connection.port, 8080);
}

#[test]
fn test_three_levels_mixed_sources() {
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=from_args", "--host=from_args"])
        .env([("DEBUG", "false"), ("TIMEOUT_MS", "1000")])
        .doc(
            "config.json",
            json!({
                "retries": 5,
                "port": 9090
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.app_name, "from_args");
    assert!(!result.debug);
    assert_eq!(result.service.timeout_ms, 1000);
    assert_eq!(result.service.retries, Some(5));
    assert_eq!(result.service.connection.host, "from_args");
    assert_eq!(result.service.connection.port, 9090);
}

#[test]
fn test_three_levels_args_shadow_json() {
    let result = AppConfig::conf_builder()
        .args([".", "--host=override", "--port=1234"])
        .env([
            ("APP_NAME", "env_app"),
            ("DEBUG", "true"),
            ("TIMEOUT_MS", "100"),
        ])
        .doc(
            "config.json",
            json!({
                "host": "should_be_shadowed",
                "port": 9999
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.service.connection.host, "override");
    assert_eq!(result.service.connection.port, 1234);
}

#[test]
fn test_three_levels_unknown_field() {
    assert_error_contains_text!(
        AppConfig::conf_builder()
            .args(["."])
            .env::<&str, &str>([])
            .doc(
                "config.json",
                json!({
                    "app_name": "myapp",
                    "debug": false,
                    "timeout_ms": 100,
                    "host": "localhost",
                    "port": 80,
                    "unknown_nested": "bad"
                })
            )
            .try_parse(),
        ["unknown field", "unknown_nested"]
    );
}

/// Four levels deep: Root -> Database -> Pool -> Credentials
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct CredentialsConfig {
    #[arg(long, env)]
    pub username: String,
    #[arg(long, env)]
    pub password: String,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct PoolConfig {
    #[arg(long, env)]
    pub min_connections: u32,
    #[arg(long, env)]
    pub max_connections: u32,
    #[conf(flatten, serde(flatten))]
    pub credentials: CredentialsConfig,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct DatabaseConfig {
    #[arg(long, env)]
    pub db_host: String,
    #[arg(long, env)]
    pub db_port: u16,
    #[conf(flatten, serde(flatten))]
    pub pool: PoolConfig,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct RootConfig {
    #[arg(long, env)]
    pub env: String,
    #[conf(flatten, serde(flatten))]
    pub database: DatabaseConfig,
}

#[test]
fn test_four_levels_all_from_json() {
    let result = RootConfig::conf_builder()
        .args(["."])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "env": "production",
                "db_host": "db.example.com",
                "db_port": 5432,
                "min_connections": 5,
                "max_connections": 100,
                "username": "admin",
                "password": "secret123"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.env, "production");
    assert_eq!(result.database.db_host, "db.example.com");
    assert_eq!(result.database.db_port, 5432);
    assert_eq!(result.database.pool.min_connections, 5);
    assert_eq!(result.database.pool.max_connections, 100);
    assert_eq!(result.database.pool.credentials.username, "admin");
    assert_eq!(result.database.pool.credentials.password, "secret123");
}

#[test]
fn test_four_levels_deepest_from_args() {
    let result = RootConfig::conf_builder()
        .args([".", "--username=cli_user", "--password=cli_pass"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "env": "staging",
                "db_host": "localhost",
                "db_port": 5432,
                "min_connections": 1,
                "max_connections": 10
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.database.pool.credentials.username, "cli_user");
    assert_eq!(result.database.pool.credentials.password, "cli_pass");
}

/// Wide hierarchy: multiple flattens at same level, each with their own nested flattens
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct LoggingConfig {
    #[arg(long, env)]
    pub log_level: String,
    #[arg(long, env)]
    pub log_format: String,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct MetricsConfig {
    #[arg(long, env)]
    pub metrics_port: u16,
    #[arg(long, env)]
    pub metrics_path: String,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ObservabilityConfig {
    #[conf(flatten, serde(flatten))]
    pub logging: LoggingConfig,
    #[conf(flatten, serde(flatten))]
    pub metrics: MetricsConfig,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ServerConfig {
    #[arg(long, env)]
    pub server_host: String,
    #[arg(long, env)]
    pub server_port: u16,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct WideConfig {
    #[arg(long, env)]
    pub name: String,
    #[conf(flatten, serde(flatten))]
    pub server: ServerConfig,
    #[conf(flatten, serde(flatten))]
    pub observability: ObservabilityConfig,
}

#[test]
fn test_wide_hierarchy_all_from_json() {
    let result = WideConfig::conf_builder()
        .args(["."])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "name": "wide_app",
                "server_host": "0.0.0.0",
                "server_port": 3000,
                "log_level": "info",
                "log_format": "json",
                "metrics_port": 9090,
                "metrics_path": "/metrics"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "wide_app");
    assert_eq!(result.server.server_host, "0.0.0.0");
    assert_eq!(result.server.server_port, 3000);
    assert_eq!(result.observability.logging.log_level, "info");
    assert_eq!(result.observability.logging.log_format, "json");
    assert_eq!(result.observability.metrics.metrics_port, 9090);
    assert_eq!(result.observability.metrics.metrics_path, "/metrics");
}

#[test]
fn test_wide_hierarchy_mixed_sources() {
    let result = WideConfig::conf_builder()
        .args([".", "--name=cli_app", "--log-level=debug"])
        .env([("SERVER_PORT", "8080"), ("METRICS_PORT", "9999")])
        .doc(
            "config.json",
            json!({
                "server_host": "127.0.0.1",
                "log_format": "text",
                "metrics_path": "/health"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "cli_app");
    assert_eq!(result.server.server_host, "127.0.0.1");
    assert_eq!(result.server.server_port, 8080);
    assert_eq!(result.observability.logging.log_level, "debug");
    assert_eq!(result.observability.logging.log_format, "text");
    assert_eq!(result.observability.metrics.metrics_port, 9999);
    assert_eq!(result.observability.metrics.metrics_path, "/health");
}

#[test]
fn test_wide_hierarchy_unknown_in_nested() {
    assert_error_contains_text!(
        WideConfig::conf_builder()
            .args(["."])
            .env::<&str, &str>([])
            .doc(
                "config.json",
                json!({
                    "name": "app",
                    "server_host": "localhost",
                    "server_port": 80,
                    "log_level": "warn",
                    "log_format": "json",
                    "metrics_port": 9090,
                    "metrics_path": "/m",
                    "log_destination": "stdout"  // unknown field in logging area
                })
            )
            .try_parse(),
        ["unknown field", "log_destination"]
    );
}

/// Diamond pattern: two paths with similar structure but unique field names
///
/// TODO: With a `serde(flatten(prefix = "..."))` feature, we could reuse the same
/// EndpointConfig struct for both primary and secondary, and the prefix would be
/// prepended to the child's serde keys during deserialization. For now, we need
/// separate structs with unique field names.
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct PrimaryEndpointConfig {
    #[arg(long, env)]
    pub primary_url: String,
    #[arg(long, env)]
    pub primary_timeout: u32,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct SecondaryEndpointConfig {
    #[arg(long, env)]
    pub secondary_url: String,
    #[arg(long, env)]
    pub secondary_timeout: u32,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct PrimaryConfig {
    #[arg(long, env)]
    pub primary_name: String,
    #[conf(flatten, serde(flatten))]
    pub endpoint: PrimaryEndpointConfig,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct SecondaryConfig {
    #[arg(long, env)]
    pub secondary_name: String,
    #[conf(flatten, serde(flatten))]
    pub endpoint: SecondaryEndpointConfig,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct DiamondConfig {
    #[arg(long, env)]
    pub id: String,
    #[conf(flatten, serde(flatten))]
    pub primary: PrimaryConfig,
    #[conf(flatten, serde(flatten))]
    pub secondary: SecondaryConfig,
}

#[test]
fn test_diamond_pattern_all_from_json() {
    let result = DiamondConfig::conf_builder()
        .args(["."])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "id": "diamond",
                "primary_name": "primary",
                "primary_url": "http://primary.example.com",
                "primary_timeout": 1000,
                "secondary_name": "secondary",
                "secondary_url": "http://secondary.example.com",
                "secondary_timeout": 2000
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.id, "diamond");
    assert_eq!(result.primary.primary_name, "primary");
    assert_eq!(
        result.primary.endpoint.primary_url,
        "http://primary.example.com"
    );
    assert_eq!(result.primary.endpoint.primary_timeout, 1000);
    assert_eq!(result.secondary.secondary_name, "secondary");
    assert_eq!(
        result.secondary.endpoint.secondary_url,
        "http://secondary.example.com"
    );
    assert_eq!(result.secondary.endpoint.secondary_timeout, 2000);
}

#[test]
fn test_diamond_pattern_mixed_sources() {
    let result = DiamondConfig::conf_builder()
        .args([".", "--primary-url=from_args", "--secondary-timeout=9999"])
        .env([("ID", "env_diamond"), ("SECONDARY_NAME", "env_secondary")])
        .doc(
            "config.json",
            json!({
                "primary_name": "json_primary",
                "primary_timeout": 500,
                "secondary_url": "http://json.example.com"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.id, "env_diamond");
    assert_eq!(result.primary.primary_name, "json_primary");
    assert_eq!(result.primary.endpoint.primary_url, "from_args");
    assert_eq!(result.primary.endpoint.primary_timeout, 500);
    assert_eq!(result.secondary.secondary_name, "env_secondary");
    assert_eq!(
        result.secondary.endpoint.secondary_url,
        "http://json.example.com"
    );
    assert_eq!(result.secondary.endpoint.secondary_timeout, 9999);
}

/// Optional fields at various levels
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct OptionalLeafConfig {
    #[arg(long, env)]
    pub required_field: String,
    #[arg(long, env)]
    pub optional_field: Option<String>,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct OptionalMiddleConfig {
    #[arg(long, env)]
    pub middle_required: String,
    #[arg(long, env)]
    pub middle_optional: Option<u32>,
    #[conf(flatten, serde(flatten))]
    pub leaf: OptionalLeafConfig,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct OptionalRootConfig {
    #[arg(long, env)]
    pub root_name: String,
    #[conf(flatten, serde(flatten))]
    pub middle: OptionalMiddleConfig,
}

#[test]
fn test_optional_fields_at_various_levels_all_present() {
    let result = OptionalRootConfig::conf_builder()
        .args(["."])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "root_name": "root",
                "middle_required": "middle",
                "middle_optional": 42,
                "required_field": "leaf",
                "optional_field": "present"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.root_name, "root");
    assert_eq!(result.middle.middle_required, "middle");
    assert_eq!(result.middle.middle_optional, Some(42));
    assert_eq!(result.middle.leaf.required_field, "leaf");
    assert_eq!(
        result.middle.leaf.optional_field,
        Some("present".to_string())
    );
}

#[test]
fn test_optional_fields_at_various_levels_none_present() {
    let result = OptionalRootConfig::conf_builder()
        .args(["."])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "root_name": "root",
                "middle_required": "middle",
                "required_field": "leaf"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.middle.middle_optional, None);
    assert_eq!(result.middle.leaf.optional_field, None);
}

#[test]
fn test_optional_fields_from_different_sources() {
    let result = OptionalRootConfig::conf_builder()
        .args([".", "--optional-field=from_args"])
        .env([("MIDDLE_OPTIONAL", "99")])
        .doc(
            "config.json",
            json!({
                "root_name": "root",
                "middle_required": "middle",
                "required_field": "leaf"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.middle.middle_optional, Some(99));
    assert_eq!(
        result.middle.leaf.optional_field,
        Some("from_args".to_string())
    );
}

/// Renames at different levels
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct RenamedLeafConfig {
    #[arg(long, env)]
    #[conf(serde(rename = "leaf_renamed"))]
    pub leaf_field: String,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct RenamedMiddleConfig {
    #[arg(long, env)]
    #[conf(serde(rename = "middle_renamed"))]
    pub middle_field: String,
    #[conf(flatten, serde(flatten))]
    pub leaf: RenamedLeafConfig,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct RenamedRootConfig {
    #[arg(long, env)]
    #[conf(serde(rename = "root_renamed"))]
    pub root_field: String,
    #[conf(flatten, serde(flatten))]
    pub middle: RenamedMiddleConfig,
}

#[test]
fn test_renames_at_all_levels() {
    let result = RenamedRootConfig::conf_builder()
        .args(["."])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "root_renamed": "root_value",
                "middle_renamed": "middle_value",
                "leaf_renamed": "leaf_value"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.root_field, "root_value");
    assert_eq!(result.middle.middle_field, "middle_value");
    assert_eq!(result.middle.leaf.leaf_field, "leaf_value");
}

#[test]
fn test_renames_args_use_original_names() {
    // Args use the original field names, not the serde renames
    let result = RenamedRootConfig::conf_builder()
        .args([
            ".",
            "--root-field=from_args",
            "--middle-field=from_args",
            "--leaf-field=from_args",
        ])
        .env::<&str, &str>([])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();

    assert_eq!(result.root_field, "from_args");
    assert_eq!(result.middle.middle_field, "from_args");
    assert_eq!(result.middle.leaf.leaf_field, "from_args");
}

#[test]
fn test_renames_args_shadow_json() {
    let result = RenamedRootConfig::conf_builder()
        .args([".", "--leaf-field=from_args"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "root_renamed": "from_json",
                "middle_renamed": "from_json",
                "leaf_renamed": "should_be_shadowed"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.root_field, "from_json");
    assert_eq!(result.middle.middle_field, "from_json");
    assert_eq!(result.middle.leaf.leaf_field, "from_args");
}

/// Empty JSON should fall back to args/env for all nested fields
#[test]
fn test_empty_json_falls_back_to_args_env() {
    let result = AppConfig::conf_builder()
        .args([".", "--app-name=app", "--host=h", "--port=1"])
        .env([("DEBUG", "true"), ("TIMEOUT_MS", "100")])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();

    assert_eq!(result.app_name, "app");
    assert!(result.debug);
    assert_eq!(result.service.timeout_ms, 100);
    assert_eq!(result.service.connection.host, "h");
    assert_eq!(result.service.connection.port, 1);
}

/// Partial JSON with rest from args/env
#[test]
fn test_partial_json_at_each_level() {
    let result = RootConfig::conf_builder()
        .args([".", "--password=secret"])
        .env([("ENV", "test"), ("MIN_CONNECTIONS", "2")])
        .doc(
            "config.json",
            json!({
                "db_host": "localhost",
                "db_port": 5432,
                "max_connections": 50,
                "username": "user"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.env, "test");
    assert_eq!(result.database.db_host, "localhost");
    assert_eq!(result.database.db_port, 5432);
    assert_eq!(result.database.pool.min_connections, 2);
    assert_eq!(result.database.pool.max_connections, 50);
    assert_eq!(result.database.pool.credentials.username, "user");
    assert_eq!(result.database.pool.credentials.password, "secret");
}
