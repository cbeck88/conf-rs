mod common;

use conf::{Conf, Subcommands};

#[derive(Conf, Debug)]
struct ConfigArgs {
    #[conf(long)]
    verbose: bool,
}

#[derive(Subcommands, Debug)]
enum Command {
    /// Run the server
    #[conf(name = "RUN")]
    Run(ConfigArgs),
    /// Stop the server
    #[conf(name = "STOP", alias = "HALT", alias = "SHUTDOWN")]
    Stop,
    /// Get status
    #[conf(name = "STATUS", alias = "STAT")]
    Status(ConfigArgs),
}

#[test]
fn test_subcommand_with_alias() {
    // Test primary name
    let wrapper =
        TestWrapper::try_parse_from::<&str, &str, &str>(vec![".", "RUN", "--verbose"], vec![])
            .unwrap();
    assert!(matches!(wrapper.command, Command::Run(_)));

    // Test alias for Stop
    let wrapper =
        TestWrapper::try_parse_from::<&str, &str, &str>(vec![".", "HALT"], vec![]).unwrap();
    assert!(matches!(wrapper.command, Command::Stop));

    // Test another alias for Stop
    let wrapper =
        TestWrapper::try_parse_from::<&str, &str, &str>(vec![".", "SHUTDOWN"], vec![]).unwrap();
    assert!(matches!(wrapper.command, Command::Stop));

    // Test alias for Status
    let wrapper =
        TestWrapper::try_parse_from::<&str, &str, &str>(vec![".", "STAT", "--verbose"], vec![])
            .unwrap();
    assert!(matches!(wrapper.command, Command::Status(_)));
}

#[test]
fn test_subcommand_names_include_aliases() {
    let names = Command::get_subcommand_names();
    assert!(names.contains(&"RUN"));
    assert!(names.contains(&"STOP"));
    assert!(names.contains(&"HALT"));
    assert!(names.contains(&"SHUTDOWN"));
    assert!(names.contains(&"STATUS"));
    assert!(names.contains(&"STAT"));
}

#[derive(Conf, Debug)]
struct TestWrapper {
    #[conf(subcommands)]
    command: Command,
}
