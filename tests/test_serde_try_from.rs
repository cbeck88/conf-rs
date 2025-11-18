//! Tests for serde(try_from = "T") feature
//!
//! This feature allows deserializing an intermediate type T and then converting
//! to the target type using TryFrom.
//!
//! ## Behavior for wrapper types
//!
//! - For `Option<Target>` fields with `try_from = "U"`:
//!   - Deserializes `Option<U>` from JSON
//!   - Converts via `.map(TryFrom::try_from)` - i.e., `Some(u)` becomes `Some(target)`
//!   - This allows the field to be optional in JSON (null or missing)
//!   - The same `TryFrom<U> for Target` impl works for both optional and required fields
//!
//! - For `Vec<Target>` fields with `try_from = "U"`:
//!   - Deserializes `Vec<U>` from JSON
//!   - Converts each element via `TryFrom::try_from`
//!   - The same `TryFrom<U> for Target` impl works for both Vec and scalar fields
//!
//! If you need more control (e.g., the conversion itself returns Option), use
//! `deserialize_with` instead.

use conf::Conf;
use serde_json::json;
use std::str::FromStr;

// A simple wrapper type that can be converted from u32
#[derive(Debug, Clone, PartialEq)]
pub struct PositiveNumber(u32);

impl TryFrom<u32> for PositiveNumber {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value > 0 {
            Ok(PositiveNumber(value))
        } else {
            Err("number must be positive")
        }
    }
}

impl FromStr for PositiveNumber {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u32 = s.parse().map_err(|e| format!("{}", e))?;
        PositiveNumber::try_from(value).map_err(|e| e.to_string())
    }
}

// Test basic try_from with parameter
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct BasicTryFrom {
    #[conf(long, serde(try_from = "u32"))]
    count: PositiveNumber,
}

#[test]
fn test_basic_try_from_success() {
    let result = BasicTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"count": 42}))
        .try_parse()
        .unwrap();
    assert_eq!(result.count.0, 42);
}

#[test]
fn test_basic_try_from_failure() {
    let result = BasicTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"count": 0}))
        .try_parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("number must be positive"),
        "error: {}",
        err_str
    );
}

// A type that converts from string to number
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedNumber(i64);

impl TryFrom<String> for ParsedNumber {
    type Error = std::num::ParseIntError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse::<i64>().map(ParsedNumber)
    }
}

impl FromStr for ParsedNumber {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<i64>().map(ParsedNumber)
    }
}

// Test try_from with string to number conversion
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct StringTryFrom {
    #[conf(long, serde(try_from = "String"))]
    value: ParsedNumber,
}

#[test]
fn test_string_try_from_success() {
    let result = StringTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"value": "123"}))
        .try_parse()
        .unwrap();
    assert_eq!(result.value.0, 123);
}

#[test]
fn test_string_try_from_failure() {
    let result = StringTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"value": "not-a-number"}))
        .try_parse();
    assert!(result.is_err());
}

// Test try_from with optional fields
// When field is Option<T> and try_from = "U", we deserialize Option<U>
// and convert via .map(TryFrom::try_from)
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct OptionalTryFrom {
    #[conf(long, serde(try_from = "u32"))]
    count: Option<PositiveNumber>,
}

#[test]
fn test_optional_try_from_with_value() {
    let result = OptionalTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"count": 42}))
        .try_parse()
        .unwrap();
    assert_eq!(result.count.as_ref().map(|p| p.0), Some(42));
}

#[test]
fn test_optional_try_from_with_null() {
    let result = OptionalTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"count": null}))
        .try_parse()
        .unwrap();
    assert!(result.count.is_none());
}

#[test]
fn test_optional_try_from_missing_field() {
    let result = OptionalTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();
    assert!(result.count.is_none());
}

#[test]
fn test_optional_try_from_conversion_failure() {
    let result = OptionalTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"count": 0}))
        .try_parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("number must be positive"),
        "error: {}",
        err_str
    );
}

// Test try_from with Vec (repeat) fields
// When field is Vec<T> and try_from = "U", we deserialize Vec<U>
// and convert each element via TryFrom
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct VecTryFrom {
    #[conf(repeat, long, serde(try_from = "u32"))]
    counts: Vec<PositiveNumber>,
}

#[test]
fn test_vec_try_from_success() {
    let result = VecTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"counts": [1, 2, 3]}))
        .try_parse()
        .unwrap();
    assert_eq!(result.counts.len(), 3);
    assert_eq!(result.counts[0].0, 1);
    assert_eq!(result.counts[1].0, 2);
    assert_eq!(result.counts[2].0, 3);
}

#[test]
fn test_vec_try_from_empty() {
    let result = VecTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"counts": []}))
        .try_parse()
        .unwrap();
    assert!(result.counts.is_empty());
}

#[test]
fn test_vec_try_from_conversion_failure() {
    let result = VecTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"counts": [1, 0, 3]}))
        .try_parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("number must be positive"),
        "error: {}",
        err_str
    );
}

#[test]
fn test_vec_try_from_cli_override() {
    let result = VecTryFrom::conf_builder()
        .args(["test", "--counts", "99", "--counts", "100"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"counts": [1, 2, 3]}))
        .try_parse()
        .unwrap();
    // CLI should override JSON
    assert_eq!(result.counts.len(), 2);
    assert_eq!(result.counts[0].0, 99);
    assert_eq!(result.counts[1].0, 100);
}

// Test try_from with flatten field (nested struct)
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct FlattenedConfig {
    #[conf(long, serde(try_from = "u32"))]
    threshold: PositiveNumber,
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ParentWithFlatten {
    #[conf(long)]
    name: String,
    #[conf(flatten)]
    config: FlattenedConfig,
}

#[test]
fn test_try_from_in_flattened_struct() {
    let result = ParentWithFlatten::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({"name": "test", "config": {"threshold": 100}}),
        )
        .try_parse()
        .unwrap();
    assert_eq!(result.name, "test");
    assert_eq!(result.config.threshold.0, 100);
}

#[test]
fn test_try_from_in_flattened_struct_failure() {
    let result = ParentWithFlatten::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({"name": "test", "config": {"threshold": 0}}),
        )
        .try_parse();
    assert!(result.is_err());
}

// Test try_from combined with CLI args (CLI should override)
#[test]
fn test_try_from_overridden_by_cli() {
    let result = BasicTryFrom::conf_builder()
        .args(["test", "--count", "99"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"count": 10}))
        .try_parse()
        .unwrap();
    // CLI value should override the JSON value
    assert_eq!(result.count.0, 99);
}

// Test try_from with rename
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct TryFromWithRename {
    #[conf(long, serde(rename = "value", try_from = "u32"))]
    count: PositiveNumber,
}

#[test]
fn test_try_from_with_rename() {
    let result = TryFromWithRename::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"value": 42}))
        .try_parse()
        .unwrap();
    assert_eq!(result.count.0, 42);
}

// Test try_from with aliases
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct TryFromWithAliases {
    #[conf(long, serde(alias = "cnt", alias = "num", try_from = "u32"))]
    count: PositiveNumber,
}

#[test]
fn test_try_from_with_alias() {
    let result = TryFromWithAliases::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"cnt": 42}))
        .try_parse()
        .unwrap();
    assert_eq!(result.count.0, 42);
}

// Test try_from with a type that needs the full path
#[derive(Debug, Clone, PartialEq)]
pub struct CustomWrapper {
    inner: i32,
}

impl TryFrom<i32> for CustomWrapper {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(CustomWrapper { inner: value })
        } else {
            Err(format!("value {} must be non-negative", value))
        }
    }
}

impl FromStr for CustomWrapper {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: i32 = s.parse().map_err(|e| format!("{}", e))?;
        CustomWrapper::try_from(value)
    }
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct FullPathTryFrom {
    #[conf(long, serde(try_from = "i32"))]
    wrapper: CustomWrapper,
}

#[test]
fn test_full_path_try_from() {
    let result = FullPathTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"wrapper": 42}))
        .try_parse()
        .unwrap();
    assert_eq!(result.wrapper.inner, 42);
}

#[test]
fn test_full_path_try_from_with_formatted_error() {
    let result = FullPathTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"wrapper": -5}))
        .try_parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("must be non-negative"),
        "error: {}",
        err_str
    );
    assert!(err_str.contains("-5"), "error: {}", err_str);
}

// Test try_from on flatten field itself
// The intermediate type must implement ConfSerde so that CLI/env shadowing works properly.
// This is different from having try_from on fields *within* a flattened struct.

// Intermediate type - implements Conf + ConfSerde
#[derive(Conf, Debug, Clone)]
#[conf(serde)]
pub struct IntermediateConfig {
    #[conf(long)]
    pub value: i32,
    #[conf(long)]
    pub name: String,
}

// Target type - implements Conf + TryFrom<IntermediateConfig>
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ValidatedConfig {
    #[conf(long)]
    pub value: i32,
    #[conf(long)]
    pub name: String,
}

impl TryFrom<IntermediateConfig> for ValidatedConfig {
    type Error = String;

    fn try_from(intermediate: IntermediateConfig) -> Result<Self, Self::Error> {
        if intermediate.value < 0 {
            return Err(format!("value {} must be non-negative", intermediate.value));
        }
        if intermediate.name.is_empty() {
            return Err("name must not be empty".to_string());
        }
        Ok(ValidatedConfig {
            value: intermediate.value,
            name: intermediate.name,
        })
    }
}

#[derive(Conf, Debug)]
#[conf(serde)]
pub struct ParentWithFlattenTryFrom {
    #[conf(long)]
    pub flag: bool,
    #[conf(flatten, serde(try_from = "IntermediateConfig"))]
    pub config: ValidatedConfig,
}

#[test]
fn test_flatten_try_from_basic() {
    let result = ParentWithFlattenTryFrom::conf_builder()
        .args(["test", "--flag"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({"config": {"value": 42, "name": "test"}}),
        )
        .try_parse()
        .unwrap();
    assert!(result.flag);
    assert_eq!(result.config.value, 42);
    assert_eq!(result.config.name, "test");
}

#[test]
fn test_flatten_try_from_cli_overrides_serde() {
    // This is the key test: CLI args should override serde values
    // because the intermediate type uses ConfSerdeSeed
    let result = ParentWithFlattenTryFrom::conf_builder()
        .args(["test", "--flag", "--value", "99", "--name", "from_cli"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({"config": {"value": 42, "name": "from_json"}}),
        )
        .try_parse()
        .unwrap();
    assert!(result.flag);
    // CLI values should win
    assert_eq!(result.config.value, 99);
    assert_eq!(result.config.name, "from_cli");
}

#[test]
fn test_flatten_try_from_partial_cli_override() {
    // Test that some fields come from CLI, others from serde
    let result = ParentWithFlattenTryFrom::conf_builder()
        .args(["test", "--flag", "--value", "77"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({"config": {"value": 42, "name": "from_json"}}),
        )
        .try_parse()
        .unwrap();
    assert!(result.flag);
    assert_eq!(result.config.value, 77); // From CLI
    assert_eq!(result.config.name, "from_json"); // From serde
}

#[test]
fn test_flatten_try_from_validation_failure() {
    let result = ParentWithFlattenTryFrom::conf_builder()
        .args(["test", "--flag"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({"config": {"value": -5, "name": "test"}}),
        )
        .try_parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("must be non-negative"),
        "error: {}",
        err_str
    );
}

#[test]
fn test_flatten_try_from_cli_only() {
    // Test that it works with CLI args only (no serde doc for the config)
    let result = ParentWithFlattenTryFrom::conf_builder()
        .args(["test", "--flag", "--value", "123", "--name", "cli_only"])
        .env::<&str, &str>([])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();
    assert!(result.flag);
    assert_eq!(result.config.value, 123);
    assert_eq!(result.config.name, "cli_only");
}

// Test try_from with default value
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct TryFromWithDefault {
    #[conf(long, default_value = "10", serde(try_from = "u32"))]
    count: PositiveNumber,
}

#[test]
fn test_try_from_with_default_json_overrides() {
    let result = TryFromWithDefault::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"count": 42}))
        .try_parse()
        .unwrap();
    assert_eq!(result.count.0, 42);
}

#[test]
fn test_try_from_with_default_uses_default() {
    let result = TryFromWithDefault::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({}))
        .try_parse()
        .unwrap();
    assert_eq!(result.count.0, 10);
}

// Test multiple fields with try_from
#[derive(Conf, Debug)]
#[conf(serde)]
pub struct MultipleTryFrom {
    #[conf(long, serde(try_from = "u32"))]
    first: PositiveNumber,
    #[conf(long, serde(try_from = "u32"))]
    second: PositiveNumber,
}

#[test]
fn test_multiple_try_from_success() {
    let result = MultipleTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"first": 1, "second": 2}))
        .try_parse()
        .unwrap();
    assert_eq!(result.first.0, 1);
    assert_eq!(result.second.0, 2);
}

#[test]
fn test_multiple_try_from_failure() {
    let result = MultipleTryFrom::conf_builder()
        .args(["test"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"first": 0, "second": 2}))
        .try_parse();
    assert!(result.is_err());
}
