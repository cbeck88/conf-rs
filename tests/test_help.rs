#![allow(dead_code)]

use conf::{Conf, Parser};

mod common;
use common::assert_multiline_eq;

#[derive(Conf)]
struct SubsystemOptions {
    /// Flag description
    #[conf(long)]
    my_flag: bool,

    /// Param description
    #[conf(long, env)]
    my_param: String,

    /// Values description
    #[conf(repeat, long, env)]
    my_list: Vec<String>,
}

#[test]
fn test_subsystem_options_help() {
    let parser_config = SubsystemOptions::get_parser_config().unwrap();
    let opts = SubsystemOptions::get_program_options().unwrap();

    let env = Default::default();
    let parser = Parser::new(parser_config, &opts, &[], &env).unwrap();

    let clap_help = parser.render_clap_help();
    let expected = &"
Usage: . [OPTIONS]

Options:
      --my-flag              Flag description
      --my-param <my_param>  Param description
                             [env MY_PARAM=]
      --my-list <my_list>    Values description
                             [env MY_LIST=]
  -h, --help                 Print help
"[1..];
    assert_multiline_eq!(&clap_help, expected);
}

#[derive(Conf)]
#[conf(about = "Launches other subsystem")]
struct OtherSubsystemOptions {
    /// Alpha
    #[conf(short)]
    alpha: bool,

    /// Beta
    #[conf(short, long)]
    beta: bool,

    /// Gamma
    #[conf(short, env)]
    gamma: bool,

    /// Delta
    #[conf(long, env)]
    delta: bool,
}

#[test]
fn test_other_subsystem_options_help() {
    let parser_config = OtherSubsystemOptions::get_parser_config().unwrap();
    let opts = OtherSubsystemOptions::get_program_options().unwrap();

    let env = Default::default();
    let parser = Parser::new(parser_config, &opts, &[], &env).unwrap();

    let clap_help = parser.render_clap_help();
    let expected = &"
Launches other subsystem

Usage: . [OPTIONS]

Options:
  -a           Alpha
  -b, --beta   Beta
  -g           Gamma
               [env GAMMA=]
      --delta  Delta
               [env DELTA=]
  -h, --help   Print help
"[1..];
    assert_multiline_eq!(&clap_help, expected);
}

/// Launches a system
#[derive(Conf)]
struct SystemOptions {
    /// Flag
    #[conf(short)]
    my_flag: bool,

    /// Other flag
    #[conf(short = 'o', long)]
    my_other_flag: bool,

    /// Param
    #[conf(long, env)]
    my_param: String,

    /// Other Param
    #[conf(long, env, default_value = "foo")]
    other_param: String,

    /// Values
    #[conf(repeat, long, env)]
    my_list: Vec<String>,

    /// Subsystem
    #[conf(flatten, prefix, help_prefix)]
    subsystem: SubsystemOptions,

    /// Other Subsystem
    #[conf(flatten)]
    other_subsystem: OtherSubsystemOptions,

    /// Required value
    #[conf(env)]
    required: String,
}

#[test]
fn test_system_options_help() {
    let parser_config = SystemOptions::get_parser_config().unwrap();
    let opts = SystemOptions::get_program_options().unwrap();

    let env = Default::default();
    let parser = Parser::new(parser_config, &opts, &[], &env).unwrap();

    let clap_help = parser.render_clap_help();
    let expected = &"
Launches a system

Usage: . [OPTIONS]

Options:
  -m                                             Flag
  -o, --my-other-flag                            Other flag
      --my-param <my_param>                      Param
                                                 [env MY_PARAM=]
      --other-param <other_param>                Other Param
                                                 [env OTHER_PARAM=]
                                                 [default: foo]
      --my-list <my_list>                        Values
                                                 [env MY_LIST=]
      --subsystem-my-flag                        Subsystem Flag description
      --subsystem-my-param <subsystem.my_param>  Subsystem Param description
                                                 [env SUBSYSTEM_MY_PARAM=]
      --subsystem-my-list <subsystem.my_list>    Subsystem Values description
                                                 [env SUBSYSTEM_MY_LIST=]
  -a                                             Alpha
  -b, --beta                                     Beta
  -g                                             Gamma
                                                 [env GAMMA=]
      --delta                                    Delta
                                                 [env DELTA=]
  -h, --help                                     Print help

Environment variables:
      <required>
          Required value
          [env: REQUIRED=]
"[1..];
    assert_multiline_eq!(&clap_help, expected);
}
