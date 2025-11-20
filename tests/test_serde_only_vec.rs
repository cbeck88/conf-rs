//! Test that serde-only Vec fields don't require FromStr implementation
//!
//! This test documents a bug where serde-only repeat fields (Vec<T>)
//! require FromStr even though they will never be parsed from CLI/env.

use serde::{Deserialize, Serialize};

// A type that has Deserialize but intentionally does NOT have FromStr
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomData {
    pub id: u32,
    pub label: String,
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_only_vec_should_not_require_fromstr() {
    use conf::Conf;
    use serde_json::json;

    #[derive(Conf, Debug)]
    #[conf(serde)]
    pub struct Config {
        #[arg(long)]
        pub name: String,

        // This is serde-only - no long, short, or env
        // It should NOT require FromStr since it will never be parsed from CLI/env
        #[arg(serde)]
        pub tags: Vec<String>,
    }

    let result = Config::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"tags": ["a", "b", "c"]}))
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "test");
    assert_eq!(result.tags, vec!["a", "b", "c"]);
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_only_optional_with_default() {
    use conf::Conf;
    use serde_json::json;

    #[derive(Conf, Debug)]
    #[conf(serde)]
    pub struct Config {
        #[arg(long)]
        pub name: String,

        // Serde-only optional field with default value
        // When serde doesn't provide the value, should use the default (Some(5))
        #[arg(serde, default_value = "5")]
        pub count: Option<u32>,

        // Serde-only optional field without default
        // When serde doesn't provide the value, should be None
        #[arg(serde)]
        pub optional_field: Option<String>,
    }

    // Test with serde providing the value
    let result = Config::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"count": 10}))
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "test");
    assert_eq!(result.count, Some(10));
    assert_eq!(result.optional_field, None);

    // Test without serde providing the value - should use default
    let result = Config::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "test");
    assert_eq!(result.count, Some(5)); // Should be Some(5), not None
    assert_eq!(result.optional_field, None);
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_only_vec_custom_type_without_fromstr() {
    use conf::Conf;
    use serde_json::json;

    #[derive(Conf, Debug)]
    #[conf(serde)]
    pub struct Config {
        #[arg(long)]
        pub name: String,

        // Serde-only repeat field with Vec<CustomData> where CustomData doesn't have FromStr
        // This should compile and work fine
        #[arg(repeat, serde)]
        pub items: Vec<CustomData>,
    }

    let result = Config::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "items": [
                    {"id": 1, "label": "first"},
                    {"id": 2, "label": "second"}
                ]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "test");
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.items[0].id, 1);
    assert_eq!(result.items[0].label, "first");
    assert_eq!(result.items[1].id, 2);
    assert_eq!(result.items[1].label, "second");

    // Test with empty document - should get empty vec
    let result = Config::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "test");
    assert_eq!(result.items.len(), 0);
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_only_repeat_custom_type_without_fromstr() {
    use conf::Conf;
    use serde_json::json;

    #[derive(Conf, Debug)]
    #[conf(serde)]
    pub struct Config {
        #[arg(long)]
        pub name: String,

        // Serde-only repeat field with CustomData (no FromStr)
        // Using #[arg(repeat, serde)] instead of just #[arg(serde)]
        #[arg(repeat, serde)]
        pub data: Vec<CustomData>,
    }

    let result = Config::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({
                "data": [
                    {"id": 10, "label": "alpha"},
                    {"id": 20, "label": "beta"}
                ]
            }),
        )
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "test");
    assert_eq!(result.data.len(), 2);
    assert_eq!(result.data[0].id, 10);
    assert_eq!(result.data[0].label, "alpha");
    assert_eq!(result.data[1].id, 20);
    assert_eq!(result.data[1].label, "beta");

    // Test without document - should get empty vec
    let result = Config::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "test");
    assert_eq!(result.data.len(), 0);
}
