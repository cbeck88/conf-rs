#![cfg(feature = "serde")]

use conf::Conf;
use serde_json::json;

/// Leaf config used in multiple places (diamond pattern)
#[derive(Conf, Debug, Clone)]
#[conf(serde)]
pub struct LeafConfig {
    #[arg(long, env)]
    pub value: i32,
    #[arg(long, env)]
    pub name: String,
}

/// Middle config B that flattens LeafConfig
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct MiddleConfigB {
    #[arg(long, env)]
    pub b_flag: bool,
    #[conf(flatten, prefix = "leaf", serde(flatten(prefix = "leaf.")))]
    pub leaf: LeafConfig,
}

/// Middle config C that flattens LeafConfig
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct MiddleConfigC {
    #[arg(long, env)]
    pub c_count: u32,
    #[conf(flatten, prefix = "leaf", serde(flatten(prefix = "leaf.")))]
    pub leaf: LeafConfig,
}

/// Top-level config A that flattens B and C (diamond pattern)
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct DiamondConfig {
    #[arg(long, env)]
    pub top_name: String,
    #[conf(flatten, prefix = "b", serde(flatten(prefix = "b.")))]
    pub config_b: MiddleConfigB,
    #[conf(flatten, prefix = "c", serde(flatten(prefix = "c.")))]
    pub config_c: MiddleConfigC,
}

#[test]
fn test_diamond_pattern_all_from_json() {
    // Diamond: A -> B -> Leaf, A -> C -> Leaf
    // With custom prefixes "b." and "c." for children, "leaf." for grandchildren
    // JSON keys: b.b_flag, b.leaf.value, b.leaf.name, c.c_count, c.leaf.value, c.leaf.name
    let result = DiamondConfig::conf_builder()
        .args([".", "--top-name=diamond"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "b.b_flag": true,
                "b.leaf.value": 100,
                "b.leaf.name": "b_leaf",
                "c.c_count": 5,
                "c.leaf.value": 200,
                "c.leaf.name": "c_leaf"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.top_name, "diamond");
    assert!(result.config_b.b_flag);
    assert_eq!(result.config_b.leaf.value, 100);
    assert_eq!(result.config_b.leaf.name, "b_leaf");
    assert_eq!(result.config_c.c_count, 5);
    assert_eq!(result.config_c.leaf.value, 200);
    assert_eq!(result.config_c.leaf.name, "c_leaf");
}

#[test]
fn test_diamond_pattern_cli_overrides() {
    // CLI args override nested prefixed JSON values
    let result = DiamondConfig::conf_builder()
        .args([
            ".",
            "--top-name=diamond",
            "--b-leaf-value=999",
            "--c-leaf-name=from_cli",
        ])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "b.b_flag": false,
                "b.leaf.value": 100,
                "b.leaf.name": "b_leaf",
                "c.c_count": 5,
                "c.leaf.value": 200,
                "c.leaf.name": "c_leaf"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.config_b.leaf.value, 999); // From CLI
    assert_eq!(result.config_b.leaf.name, "b_leaf"); // From JSON
    assert_eq!(result.config_c.leaf.value, 200); // From JSON
    assert_eq!(result.config_c.leaf.name, "from_cli"); // From CLI
}

#[test]
fn test_diamond_pattern_mixed_sources() {
    // Mix of CLI, env, and JSON at different levels
    let result = DiamondConfig::conf_builder()
        .args([".", "--top-name=diamond", "--b-b-flag"])
        .env([("C_C_COUNT", "42")])
        .doc(
            "config.json",
            json!({
                "b.leaf.value": 100,
                "b.leaf.name": "json_b",
                "c.leaf.value": 200,
                "c.leaf.name": "json_c"
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.top_name, "diamond");
    assert!(result.config_b.b_flag); // From CLI
    assert_eq!(result.config_b.leaf.value, 100); // From JSON
    assert_eq!(result.config_c.c_count, 42); // From env
    assert_eq!(result.config_c.leaf.name, "json_c"); // From JSON
}

/// Three-level nesting with prefixes
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct Level3Config {
    #[arg(long, env)]
    pub deep_value: String,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct Level2Config {
    #[arg(long, env)]
    pub mid_value: i32,
    #[conf(flatten, prefix = "deep", serde(flatten(prefix)))]
    pub level3: Level3Config,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct Level1Config {
    #[arg(long, env)]
    pub top_value: bool,
    #[conf(flatten, prefix = "mid", serde(flatten(prefix)))]
    pub level2: Level2Config,
}

#[test]
fn test_three_level_nesting_all_json() {
    // Three levels with serde(flatten(prefix)) which uses field names as prefixes:
    // - level2 field generates prefix "level2_"
    // - level3 field generates prefix "level3_"
    // So JSON keys are: level2_mid_value, level2_level3_deep_value
    let result = Level1Config::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "top_value": true,
                "level2_mid_value": 42,
                "level2_level3_deep_value": "deepest"
            }),
        )
        .try_parse()
        .unwrap();

    assert!(result.top_value);
    assert_eq!(result.level2.mid_value, 42);
    assert_eq!(result.level2.level3.deep_value, "deepest");
}

#[test]
fn test_three_level_nesting_cli_override_deepest() {
    // Override the deepest value from CLI
    // CLI uses conf prefix: --mid-deep-deep-value (from prefix = "mid" and prefix = "deep")
    let result = Level1Config::conf_builder()
        .args(["test", "--mid-deep-deep-value=from_cli"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "top_value": false,
                "level2_mid_value": 100,
                "level2_level3_deep_value": "from_json"
            }),
        )
        .try_parse()
        .unwrap();

    assert!(!result.top_value);
    assert_eq!(result.level2.mid_value, 100);
    assert_eq!(result.level2.level3.deep_value, "from_cli");
}

/// Custom prefix strings in nested structures
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ServerConfig {
    #[arg(long, env)]
    pub host: String,
    #[arg(long, env)]
    pub port: u16,
}

/// Test with prefix disambiguation using conf(flatten, prefix)
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct DisambiguatedConfig {
    #[arg(long, env)]
    pub name: String,
    #[conf(flatten, prefix = "db", serde(flatten(prefix = "db.")))]
    pub database: ServerConfig,
    #[conf(flatten, prefix = "cache", serde(flatten(prefix = "cache.")))]
    pub cache: ServerConfig,
}

#[test]
fn test_disambiguated_prefixes() {
    // Both conf prefix and serde prefix used
    let result = DisambiguatedConfig::conf_builder()
        .args([".", "--name=disambig", "--db-host=cli_db"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "db.host": "json_db",
                "db.port": 5432,
                "cache.host": "cache.example.com",
                "cache.port": 6379
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "disambig");
    assert_eq!(result.database.host, "cli_db"); // From CLI
    assert_eq!(result.database.port, 5432); // From JSON
    assert_eq!(result.cache.host, "cache.example.com");
    assert_eq!(result.cache.port, 6379);
}

#[test]
fn test_disambiguated_env_override() {
    // Env overrides with disambiguated prefixes
    let result = DisambiguatedConfig::conf_builder()
        .args([".", "--name=disambig"])
        .env([("CACHE_PORT", "11211")])
        .doc(
            "config.json",
            json!({
                "db.host": "db.example.com",
                "db.port": 5432,
                "cache.host": "cache.example.com",
                "cache.port": 6379
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.database.port, 5432);
    assert_eq!(result.cache.port, 11211); // From env
}

/// Wide diamond: multiple children sharing same grandchild type
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct SharedLeaf {
    #[arg(long, env)]
    pub id: u64,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ChildA {
    #[arg(long, env)]
    pub a_name: String,
    #[conf(flatten, prefix = "shared", serde(flatten(prefix)))]
    pub shared: SharedLeaf,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ChildB {
    #[arg(long, env)]
    pub b_name: String,
    #[conf(flatten, prefix = "shared", serde(flatten(prefix)))]
    pub shared: SharedLeaf,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ChildC {
    #[arg(long, env)]
    pub c_name: String,
    #[conf(flatten, prefix = "shared", serde(flatten(prefix)))]
    pub shared: SharedLeaf,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct WideDiamondConfig {
    #[arg(long, env)]
    pub root: String,
    #[conf(flatten, prefix = "a", serde(flatten(prefix)))]
    pub child_a: ChildA,
    #[conf(flatten, prefix = "b", serde(flatten(prefix)))]
    pub child_b: ChildB,
    #[conf(flatten, prefix = "c", serde(flatten(prefix)))]
    pub child_c: ChildC,
}

#[test]
fn test_wide_diamond_all_json() {
    // Wide diamond with three children, each with shared leaf
    // serde(flatten(prefix)) uses field names:
    // - child_a -> prefix "child_a_"
    // - child_b -> prefix "child_b_"
    // - child_c -> prefix "child_c_"
    // - shared -> prefix "shared_"
    let result = WideDiamondConfig::conf_builder()
        .args([".", "--root=wide"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "child_a_a_name": "child_a",
                "child_a_shared_id": 1,
                "child_b_b_name": "child_b",
                "child_b_shared_id": 2,
                "child_c_c_name": "child_c",
                "child_c_shared_id": 3
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.root, "wide");
    assert_eq!(result.child_a.a_name, "child_a");
    assert_eq!(result.child_a.shared.id, 1);
    assert_eq!(result.child_b.b_name, "child_b");
    assert_eq!(result.child_b.shared.id, 2);
    assert_eq!(result.child_c.c_name, "child_c");
    assert_eq!(result.child_c.shared.id, 3);
}

#[test]
fn test_wide_diamond_partial_cli() {
    // Override some shared ids from CLI
    let result = WideDiamondConfig::conf_builder()
        .args([".", "--root=wide", "--a-shared-id=100", "--c-shared-id=300"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "child_a_a_name": "child_a",
                "child_a_shared_id": 1,
                "child_b_b_name": "child_b",
                "child_b_shared_id": 2,
                "child_c_c_name": "child_c",
                "child_c_shared_id": 3
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.child_a.shared.id, 100); // From CLI
    assert_eq!(result.child_b.shared.id, 2); // From JSON
    assert_eq!(result.child_c.shared.id, 300); // From CLI
}
