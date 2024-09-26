//use assert_matches::assert_matches;
use conf::{Conf, Subcommands};

//mod common;
//use common::*;

#[derive(Conf)]
struct AConfig {
    #[arg(short)]
    f: bool,

    #[conf(subcommands)]
    command: Command,
}

#[derive(Subcommands)]
enum Command {
    GiveStick(GiveStickConfig),
    DontGiveStick(DontGiveStickConfig),
}

#[derive(Conf)]
struct GiveStickConfig {
    #[conf(long)]
    times: u16,
}

#[derive(Conf)]
struct DontGiveStickConfig {
    #[conf(short)]
    o: bool,
}

#[test]
fn test_required_subcommands() {
    let result = AConfig::try_parse_from::<&str, &str, &str>(vec!["."], vec![]);
    assert!(result.is_err());

    let result =
        AConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick"], vec![]).unwrap();

    assert!(!result.f);
    let Command::DontGiveStick(result) = result.command else {
        panic!("Unexpected enum val")
    };
    assert!(!result.o);

    let result =
        AConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick", "-o"], vec![])
            .unwrap();

    assert!(!result.f);
    let Command::DontGiveStick(result) = result.command else {
        panic!("Unexpected enum val")
    };
    assert!(result.o);

    let result =
        AConfig::try_parse_from::<&str, &str, &str>(vec![".", "-o", "dont-give-stick"], vec![]);
    assert!(result.is_err());

    let result = AConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-o", "dont-give-stick", "-f"],
        vec![],
    );
    assert!(result.is_err());

    let result =
        AConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick", "-f"], vec![]);
    assert!(result.is_err());

    let result =
        AConfig::try_parse_from::<&str, &str, &str>(vec![".", "-f", "dont-give-stick"], vec![])
            .unwrap();

    assert!(result.f);
    let Command::DontGiveStick(result) = result.command else {
        panic!("Unexpected enum val")
    };
    assert!(!result.o);

    let result = AConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-f", "dont-give-stick", "-o"],
        vec![],
    )
    .unwrap();

    assert!(result.f);
    let Command::DontGiveStick(result) = result.command else {
        panic!("Unexpected enum val")
    };
    assert!(result.o);

    let result = AConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-f", "give-stick", "--times=16"],
        vec![],
    )
    .unwrap();

    assert!(result.f);
    let Command::GiveStick(result) = result.command else {
        panic!("Unexpected enum val")
    };
    assert_eq!(result.times, 16);

    let result = AConfig::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "-f",
            "give-stick",
            "--times=16",
            "dont-give-stick",
            "-o",
        ],
        vec![],
    );
    assert!(result.is_err());
}

// Same thing but now subcommands are optional
#[derive(Conf)]
struct BConfig {
    #[arg(short)]
    f: bool,

    #[conf(subcommands)]
    command: Option<Command>,
}

#[test]
fn test_optional_subcommands() {
    let result = BConfig::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert!(!result.f);
    assert!(result.command.is_none());

    let result = BConfig::try_parse_from::<&str, &str, &str>(vec![".", "-f"], vec![]).unwrap();
    assert!(result.f);
    assert!(result.command.is_none());

    let result = BConfig::try_parse_from::<&str, &str, &str>(vec![".", "-o"], vec![]);
    assert!(result.is_err());

    let result =
        BConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick"], vec![]).unwrap();

    assert!(!result.f);
    let Command::DontGiveStick(result) = result.command.unwrap() else {
        panic!("Unexpected enum val")
    };
    assert!(!result.o);

    let result =
        BConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick", "-o"], vec![])
            .unwrap();

    assert!(!result.f);
    let Command::DontGiveStick(result) = result.command.unwrap() else {
        panic!("Unexpected enum val")
    };
    assert!(result.o);

    let result =
        BConfig::try_parse_from::<&str, &str, &str>(vec![".", "-o", "dont-give-stick"], vec![]);
    assert!(result.is_err());

    let result = BConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-o", "dont-give-stick", "-f"],
        vec![],
    );
    assert!(result.is_err());

    let result =
        BConfig::try_parse_from::<&str, &str, &str>(vec![".", "dont-give-stick", "-f"], vec![]);
    assert!(result.is_err());

    let result =
        BConfig::try_parse_from::<&str, &str, &str>(vec![".", "-f", "dont-give-stick"], vec![])
            .unwrap();

    assert!(result.f);
    let Command::DontGiveStick(result) = result.command.unwrap() else {
        panic!("Unexpected enum val")
    };
    assert!(!result.o);

    let result = BConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-f", "dont-give-stick", "-o"],
        vec![],
    )
    .unwrap();

    assert!(result.f);
    let Command::DontGiveStick(result) = result.command.unwrap() else {
        panic!("Unexpected enum val")
    };
    assert!(result.o);

    let result = BConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-f", "give-stick", "--times=16"],
        vec![],
    )
    .unwrap();

    assert!(result.f);
    let Command::GiveStick(result) = result.command.unwrap() else {
        panic!("Unexpected enum val")
    };
    assert_eq!(result.times, 16);

    let result = BConfig::try_parse_from::<&str, &str, &str>(
        vec![
            ".",
            "-f",
            "give-stick",
            "--times=16",
            "dont-give-stick",
            "-o",
        ],
        vec![],
    );
    assert!(result.is_err());
}

#[derive(Conf)]
struct DConfig {
    #[arg(short)]
    x: bool,

    #[conf(subcommands)]
    command: DCommand,
}

#[derive(Subcommands)]
enum DCommand {
    FrozenLake(AConfig),
    WildDog(BConfig),
}

#[test]
fn test_nested_subcommands() {
    let result = DConfig::try_parse_from::<&str, &str, &str>(vec!["."], vec![]);
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(vec![".", "frozen-lake"], vec![]);
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "frozen-lake", "dont-give-stick"],
        vec![],
    )
    .unwrap();
    assert!(!result.x);

    let DCommand::FrozenLake(a) = result.command else {
        panic!("Unexpected enum value")
    };
    assert!(!a.f);
    let Command::DontGiveStick(r) = a.command else {
        panic!("Unexpected enum value")
    };
    assert!(!r.o);

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-x", "frozen-lake", "dont-give-stick", "-o"],
        vec![],
    )
    .unwrap();
    assert!(result.x);

    let DCommand::FrozenLake(a) = result.command else {
        panic!("Unexpected enum value")
    };
    assert!(!a.f);
    let Command::DontGiveStick(r) = a.command else {
        panic!("Unexpected enum value")
    };
    assert!(r.o);

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "frozen-lake", "-x", "dont-give-stick"],
        vec![],
    );
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "frozen-lake", "dont-give-stick", "-x"],
        vec![],
    );
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "frozen-lake", "-o", "dont-give-stick"],
        vec![],
    );
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "-o", "frozen-lake", "dont-give-stick"],
        vec![],
    );
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "wild-dog", "dont-give-stick"],
        vec![],
    )
    .unwrap();
    assert!(!result.x);
    let DCommand::WildDog(b) = result.command else {
        panic!("Unexpected enum value")
    };
    assert!(!b.f);
    let Command::DontGiveStick(r) = b.command.unwrap() else {
        panic!("Unexpected enum value")
    };
    assert!(!r.o);

    let result =
        DConfig::try_parse_from::<&str, &str, &str>(vec![".", "wild-dog", "give-stick"], vec![]);
    assert!(result.is_err());

    let result = DConfig::try_parse_from::<&str, &str, &str>(
        vec![".", "wild-dog", "give-stick", "--times", "9"],
        vec![],
    )
    .unwrap();
    assert!(!result.x);
    let DCommand::WildDog(b) = result.command else {
        panic!("Unexpected enum value")
    };
    assert!(!b.f);
    let Command::GiveStick(r) = b.command.unwrap() else {
        panic!("Unexpected enum value")
    };
    assert_eq!(r.times, 9);
}
