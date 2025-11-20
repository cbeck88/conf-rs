//! Tests for the config logger introspection feature

use conf::{Conf, ConfigEvent, ValueSource};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug)]
struct LoggedEvent {
    id: String,
    source_type: String,
    source_detail: Option<String>,
}

/// Test basic flag, parameter, and repeat with various sources
#[derive(Conf, Debug)]
pub struct BasicConfig {
    #[arg(flag, long)]
    pub verbose: bool,

    #[arg(long, env)]
    pub name: String,

    #[arg(long, env, default_value = "42")]
    pub count: i32,

    #[arg(repeat, long)]
    pub tags: Vec<String>,
}

#[test]
fn test_config_logger_all_from_args() {
    let events = RefCell::new(Vec::new());

    let logger = |event: &dyn ConfigEvent| {
        let id = event.program_option().id().to_string();
        let (source_type, source_detail) = match event.value_source() {
            ValueSource::Args { .. } => ("Args".to_string(), None),
            ValueSource::Env { var, .. } => ("Env".to_string(), Some(var.to_string())),
            ValueSource::Document { name, .. } => ("Document".to_string(), Some(name.to_string())),
            ValueSource::Default { .. } => ("Default".to_string(), None),
            _ => panic!("Unexpected value source"),
        };

        events.borrow_mut().push(LoggedEvent {
            id,
            source_type,
            source_detail,
        });
    };

    let result = BasicConfig::conf_builder()
        .config_logger(logger)
        .args([
            ".",
            "--verbose",
            "--name=test",
            "--count=100",
            "--tags=a",
            "--tags=b",
        ])
        .env::<&str, &str>([])
        .try_parse()
        .unwrap();

    assert!(result.verbose);
    assert_eq!(result.name, "test");
    assert_eq!(result.count, 100);
    assert_eq!(result.tags, vec!["a", "b"]);

    let logged = events.borrow();

    // Should have exactly 4 events (one per field)
    assert_eq!(
        logged.len(),
        4,
        "Expected 4 logged events, got {}",
        logged.len()
    );

    // Check verbose flag from args
    let verbose_event = logged
        .iter()
        .find(|e| e.id == "verbose")
        .expect("verbose event");
    assert_eq!(verbose_event.source_type, "Args");
    assert_eq!(verbose_event.source_detail, None);

    // Check name from args
    let name_event = logged.iter().find(|e| e.id == "name").expect("name event");
    assert_eq!(name_event.source_type, "Args");
    assert_eq!(name_event.source_detail, None);

    // Check count from args
    let count_event = logged
        .iter()
        .find(|e| e.id == "count")
        .expect("count event");
    assert_eq!(count_event.source_type, "Args");
    assert_eq!(count_event.source_detail, None);

    // Check tags from args
    let tags_event = logged.iter().find(|e| e.id == "tags").expect("tags event");
    assert_eq!(tags_event.source_type, "Args");
    assert_eq!(tags_event.source_detail, None);
}

#[test]
fn test_config_logger_from_env() {
    let events = RefCell::new(Vec::new());

    let logger = |event: &dyn ConfigEvent| {
        let id = event.program_option().id().to_string();
        let (source_type, source_detail) = match event.value_source() {
            ValueSource::Args { .. } => ("Args".to_string(), None),
            ValueSource::Env { var, .. } => ("Env".to_string(), Some(var.to_string())),
            ValueSource::Document { name, .. } => ("Document".to_string(), Some(name.to_string())),
            ValueSource::Default { .. } => ("Default".to_string(), None),
            _ => panic!("Unexpected value source"),
        };

        events.borrow_mut().push(LoggedEvent {
            id,
            source_type,
            source_detail,
        });
    };

    let result = BasicConfig::conf_builder()
        .args(["."])
        .env([("NAME", "from-env"), ("COUNT", "99")])
        .config_logger(logger)
        .try_parse()
        .unwrap();

    assert!(!result.verbose); // Not set
    assert_eq!(result.name, "from-env");
    assert_eq!(result.count, 99);
    assert_eq!(result.tags, Vec::<String>::new());

    let logged = events.borrow();

    // Should have exactly 4 events (verbose gets false, tags gets empty vec, name and count from env)
    assert_eq!(
        logged.len(),
        4,
        "Expected 4 logged events, got {}",
        logged.len()
    );

    // Check verbose flag (default false)
    let verbose_event = logged
        .iter()
        .find(|e| e.id == "verbose")
        .expect("verbose event");
    assert_eq!(verbose_event.source_type, "Default");

    // Check name from env
    let name_event = logged.iter().find(|e| e.id == "name").expect("name event");
    assert_eq!(name_event.source_type, "Env");
    assert_eq!(name_event.source_detail, Some("NAME".to_string()));

    // Check count from env
    let count_event = logged
        .iter()
        .find(|e| e.id == "count")
        .expect("count event");
    assert_eq!(count_event.source_type, "Env");
    assert_eq!(count_event.source_detail, Some("COUNT".to_string()));

    // Check tags (default empty vec)
    let tags_event = logged.iter().find(|e| e.id == "tags").expect("tags event");
    assert_eq!(tags_event.source_type, "Default");
}

#[test]
fn test_config_logger_from_default() {
    let events = RefCell::new(Vec::new());

    let logger = |event: &dyn ConfigEvent| {
        let id = event.program_option().id().to_string();
        let (source_type, source_detail) = match event.value_source() {
            ValueSource::Args { .. } => ("Args".to_string(), None),
            ValueSource::Env { var, .. } => ("Env".to_string(), Some(var.to_string())),
            ValueSource::Document { name, .. } => ("Document".to_string(), Some(name.to_string())),
            ValueSource::Default { .. } => ("Default".to_string(), None),
            _ => panic!("Unexpected value source"),
        };

        events.borrow_mut().push(LoggedEvent {
            id,
            source_type,
            source_detail,
        });
    };

    let result = BasicConfig::conf_builder()
        .args([".", "--name=test"])
        .env::<&str, &str>([])
        .config_logger(logger)
        .try_parse()
        .unwrap();

    assert!(!result.verbose);
    assert_eq!(result.name, "test");
    assert_eq!(result.count, 42); // default value
    assert_eq!(result.tags, Vec::<String>::new());

    let logged = events.borrow();

    assert_eq!(logged.len(), 4);

    // Check count from default
    let count_event = logged
        .iter()
        .find(|e| e.id == "count")
        .expect("count event");
    assert_eq!(count_event.source_type, "Default");
    assert_eq!(count_event.source_detail, None);
}

#[cfg(feature = "serde")]
#[test]
fn test_config_logger_from_serde() {
    use serde_json::json;

    #[derive(Conf, Debug)]
    #[conf(serde)]
    pub struct SerdeConfig {
        #[arg(long, serde)]
        pub name: String,

        #[arg(long, serde, default_value = "10")]
        pub count: i32,
    }

    let events = RefCell::new(Vec::new());

    let logger = |event: &dyn ConfigEvent| {
        let id = event.program_option().id().to_string();
        let (source_type, source_detail) = match event.value_source() {
            ValueSource::Args { .. } => ("Args".to_string(), None),
            ValueSource::Env { var, .. } => ("Env".to_string(), Some(var.to_string())),
            ValueSource::Document { name, .. } => ("Document".to_string(), Some(name.to_string())),
            ValueSource::Default { .. } => ("Default".to_string(), None),
            _ => panic!("Unexpected value source"),
        };

        events.borrow_mut().push(LoggedEvent {
            id,
            source_type,
            source_detail,
        });
    };

    let result = SerdeConfig::conf_builder()
        .args(["."])
        .env::<&str, &str>([])
        .doc("config.json", json!({"name": "from-doc", "count": 99}))
        .config_logger(logger)
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "from-doc");
    assert_eq!(result.count, 99);

    let logged = events.borrow();

    // Fields with CLI/env sources are logged even when values come from serde
    // because they go through ConfContext
    assert_eq!(logged.len(), 2);

    // Check name from document
    let name_event = logged.iter().find(|e| e.id == "name").expect("name event");
    assert_eq!(name_event.source_type, "Document");
    assert_eq!(name_event.source_detail, Some("config.json".to_string()));

    // Check count from document
    let count_event = logged
        .iter()
        .find(|e| e.id == "count")
        .expect("count event");
    assert_eq!(count_event.source_type, "Document");
    assert_eq!(count_event.source_detail, Some("config.json".to_string()));
}

#[cfg(feature = "serde")]
#[test]
fn test_config_logger_args_shadow_serde() {
    use serde_json::json;

    #[derive(Conf, Debug)]
    #[conf(serde)]
    pub struct SerdeConfig {
        #[arg(long, serde)]
        pub name: String,

        #[arg(long, serde)]
        pub count: i32,
    }

    let events = RefCell::new(Vec::new());

    let logger = |event: &dyn ConfigEvent| {
        let id = event.program_option().id().to_string();
        let (source_type, source_detail) = match event.value_source() {
            ValueSource::Args { .. } => ("Args".to_string(), None),
            ValueSource::Env { var, .. } => ("Env".to_string(), Some(var.to_string())),
            ValueSource::Document { name, .. } => ("Document".to_string(), Some(name.to_string())),
            ValueSource::Default { .. } => ("Default".to_string(), None),
            _ => panic!("Unexpected value source"),
        };

        events.borrow_mut().push(LoggedEvent {
            id,
            source_type,
            source_detail,
        });
    };

    let result = SerdeConfig::conf_builder()
        .args([".", "--name=from-args"])
        .env::<&str, &str>([])
        .doc("config.json", json!({"name": "from-doc", "count": 99}))
        .config_logger(logger)
        .try_parse()
        .unwrap();

    assert_eq!(result.name, "from-args");
    assert_eq!(result.count, 99);

    let logged = events.borrow();

    assert_eq!(logged.len(), 2);

    // Check name from args (shadows document)
    let name_event = logged.iter().find(|e| e.id == "name").expect("name event");
    assert_eq!(name_event.source_type, "Args");
    assert_eq!(name_event.source_detail, None);

    // Check count from document
    let count_event = logged
        .iter()
        .find(|e| e.id == "count")
        .expect("count event");
    assert_eq!(count_event.source_type, "Document");
    assert_eq!(count_event.source_detail, Some("config.json".to_string()));
}

#[test]
fn test_config_logger_with_flattened() {
    #[derive(Conf, Debug)]
    pub struct Inner {
        #[arg(long)]
        pub port: u16,
    }

    #[derive(Conf, Debug)]
    pub struct Outer {
        #[arg(long)]
        pub host: String,

        #[conf(flatten)]
        pub server: Inner,
    }

    let events = RefCell::new(Vec::new());

    let logger = |event: &dyn ConfigEvent| {
        let id = event.program_option().id().to_string();
        let (source_type, source_detail) = match event.value_source() {
            ValueSource::Args { .. } => ("Args".to_string(), None),
            ValueSource::Env { var, .. } => ("Env".to_string(), Some(var.to_string())),
            ValueSource::Document { name, .. } => ("Document".to_string(), Some(name.to_string())),
            ValueSource::Default { .. } => ("Default".to_string(), None),
            _ => panic!("Unexpected value source"),
        };

        events.borrow_mut().push(LoggedEvent {
            id,
            source_type,
            source_detail,
        });
    };

    let result = Outer::conf_builder()
        .args([".", "--host=localhost", "--port=8080"])
        .env::<&str, &str>([])
        .config_logger(logger)
        .try_parse()
        .unwrap();

    assert_eq!(result.host, "localhost");
    assert_eq!(result.server.port, 8080);

    let logged = events.borrow();

    assert_eq!(logged.len(), 2);

    // Check host
    let host_event = logged.iter().find(|e| e.id == "host").expect("host event");
    assert_eq!(host_event.source_type, "Args");

    // Check port - should have flattened id
    let port_event = logged
        .iter()
        .find(|e| e.id == "server.port")
        .expect("port event");
    assert_eq!(port_event.source_type, "Args");
}

#[test]
fn test_config_logger_program_option_metadata() {
    let events = RefCell::new(Vec::new());

    let logger = |event: &dyn ConfigEvent| {
        let opt = event.program_option();
        let id = opt.id().to_string();

        // Store metadata we can verify
        events.borrow_mut().push((
            id,
            opt.short_form(),
            opt.long_form().map(|l| l.to_string()),
            opt.env_form().map(|e| e.to_string()),
            opt.is_positional(),
            opt.has_serde_source(),
            opt.is_required(),
        ));
    };

    let _result = BasicConfig::conf_builder()
        .args([".", "--verbose", "--name=test", "--count=100"])
        .env::<&str, &str>([])
        .config_logger(logger)
        .try_parse()
        .unwrap();

    let logged = events.borrow();

    // Verify metadata for verbose flag
    let verbose = logged
        .iter()
        .find(|(id, ..)| id == "verbose")
        .expect("verbose");
    assert_eq!(verbose.1, None); // no short form
    assert_eq!(verbose.2, Some("verbose".to_string())); // long form
    assert_eq!(verbose.3, None); // no env
    assert_eq!(verbose.4, false); // not positional
    assert_eq!(verbose.5, false); // no serde source
    assert_eq!(verbose.6, false); // not required

    // Verify metadata for name parameter
    let name = logged.iter().find(|(id, ..)| id == "name").expect("name");
    assert_eq!(name.1, None); // no short form
    assert_eq!(name.2, Some("name".to_string())); // long form
    assert_eq!(name.3, Some("NAME".to_string())); // has env
    assert_eq!(name.4, false); // not positional
    assert_eq!(name.5, false); // no serde source
    assert_eq!(name.6, true); // required
}

#[test]
fn test_config_logger_exactly_one_event_per_option() {
    let event_counts = RefCell::new(HashMap::new());

    let logger = |event: &dyn ConfigEvent| {
        let id = event.program_option().id().to_string();
        *event_counts.borrow_mut().entry(id).or_insert(0) += 1;
    };

    let _result = BasicConfig::conf_builder()
        .args([
            ".",
            "--verbose",
            "--name=test",
            "--tags=a",
            "--tags=b",
            "--tags=c",
        ])
        .env([("COUNT", "50")])
        .config_logger(logger)
        .try_parse()
        .unwrap();

    let counts = event_counts.borrow();

    // Each option should be logged exactly once
    assert_eq!(counts.get("verbose"), Some(&1));
    assert_eq!(counts.get("name"), Some(&1));
    assert_eq!(counts.get("count"), Some(&1));
    assert_eq!(counts.get("tags"), Some(&1)); // Even though multiple values, logged once

    // Total of 4 options
    assert_eq!(counts.len(), 4);
}

#[cfg(feature = "serde")]
#[test]
fn test_config_logger_serde_only_fields() {
    use serde_json::json;

    #[derive(Conf, Debug)]
    #[conf(serde)]
    pub struct SerdeOnlyConfig {
        #[arg(long)]
        pub cli_field: String,

        #[arg(serde)]
        pub serde_only_param: i32,

        #[arg(serde)]
        pub serde_only_flag: bool,
    }

    let events = RefCell::new(Vec::new());

    let logger = |event: &dyn ConfigEvent| {
        let id = event.program_option().id().to_string();
        let (source_type, source_detail) = match event.value_source() {
            ValueSource::Args { .. } => ("Args".to_string(), None),
            ValueSource::Env { var, .. } => ("Env".to_string(), Some(var.to_string())),
            ValueSource::Document { name, .. } => ("Document".to_string(), Some(name.to_string())),
            ValueSource::Default { .. } => ("Default".to_string(), None),
            _ => panic!("Unexpected value source"),
        };

        events.borrow_mut().push(LoggedEvent {
            id,
            source_type,
            source_detail,
        });
    };

    let result = SerdeOnlyConfig::conf_builder()
        .args([".", "--cli-field=test"])
        .env::<&str, &str>([])
        .doc(
            "config.json",
            json!({"serde_only_param": 42, "serde_only_flag": true}),
        )
        .config_logger(logger)
        .try_parse()
        .unwrap();

    assert_eq!(result.cli_field, "test");
    assert_eq!(result.serde_only_param, 42);
    assert!(result.serde_only_flag);

    let logged = events.borrow();

    // All three fields should be logged - even serde-only fields go through
    // the config logger when they receive values from the document
    assert_eq!(logged.len(), 3);

    // cli_field from args
    let cli_event = logged
        .iter()
        .find(|e| e.id == "cli_field")
        .expect("cli_field");
    assert_eq!(cli_event.source_type, "Args");

    // serde_only_param from document
    let param_event = logged
        .iter()
        .find(|e| e.id == "serde_only_param")
        .expect("serde_only_param");
    assert_eq!(param_event.source_type, "Document");
    assert_eq!(param_event.source_detail, Some("config.json".to_string()));

    // serde_only_flag from document
    let flag_event = logged
        .iter()
        .find(|e| e.id == "serde_only_flag")
        .expect("serde_only_flag");
    assert_eq!(flag_event.source_type, "Document");
    assert_eq!(flag_event.source_detail, Some("config.json".to_string()));
}

#[cfg(feature = "serde")]
#[test]
fn test_config_logger_repeat_with_serde() {
    use serde_json::json;

    #[derive(Conf, Debug)]
    #[conf(serde)]
    pub struct RepeatSerdeConfig {
        #[arg(repeat, long, serde)]
        pub tags: Vec<String>,

        #[arg(repeat, long, env, serde)]
        pub items: Vec<String>,
    }

    let events = RefCell::new(Vec::new());

    let logger = |event: &dyn ConfigEvent| {
        let id = event.program_option().id().to_string();
        let (source_type, source_detail) = match event.value_source() {
            ValueSource::Args { .. } => ("Args".to_string(), None),
            ValueSource::Env { var, .. } => ("Env".to_string(), Some(var.to_string())),
            ValueSource::Document { name, .. } => ("Document".to_string(), Some(name.to_string())),
            ValueSource::Default { .. } => ("Default".to_string(), None),
            _ => panic!("Unexpected value source"),
        };

        events.borrow_mut().push(LoggedEvent {
            id,
            source_type,
            source_detail,
        });
    };

    let result = RepeatSerdeConfig::conf_builder()
        .args(["."])
        .env::<&str, &str>([])
        .doc("config.json", json!({"tags": ["x", "y", "z"], "items": ["p", "q"]}))
        .config_logger(logger)
        .try_parse()
        .unwrap();

    assert_eq!(result.tags, vec!["x", "y", "z"]);
    assert_eq!(result.items, vec!["p", "q"]);

    let logged = events.borrow();

    // Should have exactly 2 events (one per field)
    assert_eq!(logged.len(), 2, "Expected 2 logged events, got {}", logged.len());

    // Check tags from document
    let tags_event = logged.iter().find(|e| e.id == "tags").expect("tags event");
    assert_eq!(tags_event.source_type, "Document");
    assert_eq!(tags_event.source_detail, Some("config.json".to_string()));

    // Check items from document
    let items_event = logged.iter().find(|e| e.id == "items").expect("items event");
    assert_eq!(items_event.source_type, "Document");
    assert_eq!(items_event.source_detail, Some("config.json".to_string()));
}
