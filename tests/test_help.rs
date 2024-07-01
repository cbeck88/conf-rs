#![allow(dead_code)]

use conf::{Conf, ParseType, Parser};
use std::str::from_utf8;

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
fn test_subsystem_options_print() {
    let (_parser_config, opts) = SubsystemOptions::get_program_options().unwrap();

    let mut iter = opts.iter();

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Flag);
    assert_eq!(opt.long_form.as_deref(), Some("my-flag"));
    let mut buffer = Vec::<u8>::new();
    opt.print(&mut buffer, None).unwrap();
    let expected = &"
      --my-flag
          Flag description
"[1..];
    assert_multiline_eq!(from_utf8(&buffer).unwrap(), expected);

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Parameter);
    assert_eq!(opt.long_form.as_deref(), Some("my-param"));
    let mut buffer = Vec::<u8>::new();
    opt.print(&mut buffer, None).unwrap();
    let expected = &"
      --my-param <MY_PARAM>
          Param description
          [env: MY_PARAM]
"[1..];
    assert_multiline_eq!(from_utf8(&buffer).unwrap(), expected);

    let opt = iter.next().unwrap();
    assert_eq!(opt.parse_type, ParseType::Repeat);
    assert_eq!(opt.long_form.as_deref(), Some("my-list"));
    let mut buffer = Vec::<u8>::new();
    opt.print(&mut buffer, None).unwrap();
    let expected = &"
      --my-list <MY_LIST>
          Values description
          [env: MY_LIST]
"[1..];
    assert_multiline_eq!(from_utf8(&buffer).unwrap(), expected);

    assert_eq!(iter.next(), None);
}

#[test]
fn test_subsystem_options_help() {
    let (parser_config, opts) = SubsystemOptions::get_program_options().unwrap();

    let env = Default::default();
    let parser = Parser::new(parser_config, &opts, &env).unwrap();

    let mut buffer = Vec::<u8>::new();
    parser
        .render_help(&mut buffer, &("subsystem".into()))
        .unwrap();
    let expected = &"
Usage: subsystem [FLAGS] [OPTIONS] --my-param <MY_PARAM>

Options:
      --my-param <MY_PARAM>
          Param description
          [env: MY_PARAM=]
      --my-list <MY_LIST>
          Values description
          [env: MY_LIST=]

Flags:
      --my-flag
          Flag description
  -h, --help
          Print help
"[1..];
    assert_multiline_eq!(from_utf8(&buffer).unwrap(), expected);
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
    let (parser_config, opts) = OtherSubsystemOptions::get_program_options().unwrap();

    let env = Default::default();
    let parser = Parser::new(parser_config, &opts, &env).unwrap();

    let mut buffer = Vec::<u8>::new();
    parser
        .render_help(&mut buffer, &("other_subsystem".into()))
        .unwrap();
    let expected = &"
Launches other subsystem

Usage: other_subsystem [FLAGS]

Flags:
  -a
          Alpha
  -b, --beta
          Beta
  -g
          Gamma
          [env: GAMMA=]
      --delta
          Delta
          [env: DELTA=]
  -h, --help
          Print help
"[1..];
    assert_multiline_eq!(from_utf8(&buffer).unwrap(), expected);
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
    let (parser_config, opts) = SystemOptions::get_program_options().unwrap();

    let env = Default::default();
    let parser = Parser::new(parser_config, &opts, &env).unwrap();

    let mut buffer = Vec::<u8>::new();
    parser.render_help(&mut buffer, &("system".into())).unwrap();
    let expected = &"
Launches a system

Usage: system [FLAGS] [OPTIONS] --my-param <MY_PARAM> --subsystem-my-param <SUBSYSTEM_MY_PARAM>

Options:
      --my-param <MY_PARAM>
          Param
          [env: MY_PARAM=]
      --subsystem-my-param <SUBSYSTEM_MY_PARAM>
          Subsystem Param description
          [env: SUBSYSTEM_MY_PARAM=]
      --other-param <OTHER_PARAM>
          Other Param
          [env: OTHER_PARAM=]
          [default: foo]
      --my-list <MY_LIST>
          Values
          [env: MY_LIST=]
      --subsystem-my-list <SUBSYSTEM_MY_LIST>
          Subsystem Values description
          [env: SUBSYSTEM_MY_LIST=]

Flags:
  -m
          Flag
  -o, --my-other-flag
          Other flag
      --subsystem-my-flag
          Subsystem Flag description
  -a
          Alpha
  -b, --beta
          Beta
  -g
          Gamma
          [env: GAMMA=]
      --delta
          Delta
          [env: DELTA=]
  -h, --help
          Print help

Required env:

          Required value
          [env: REQUIRED=]
"[1..];
    assert_multiline_eq!(from_utf8(&buffer).unwrap(), expected);
}
