use crate::{Error, ParseType, ParsedEnv, ProgramOption};
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::ffi::OsString;

/// Top-level parser config
#[derive(Clone, Debug, Default)]
pub struct ParserConfig {
    /// An optional top-level specified about string
    pub about: Option<&'static str>,
    /// A name that should be used for the binary
    pub name: &'static str,
    /// True if help flags should not be automatically generated
    pub no_help_flag: bool,
}

/// A parser which tries to parse args, matching them to a list of ProgramOptions.
#[allow(unused)]
pub struct Parser<'a> {
    parser_config: ParserConfig,
    options: Vec<&'a ProgramOption>,
    env: &'a ParsedEnv,
    command: Command,
}

impl<'a> Parser<'a> {
    /// Create a parser from top-level parser config and a list of program options
    /// This parser doesn't consider env at all when parsing, but does use env when rendering help.
    /// It also tries to fail with a long list if any required options are not supplied, and checks against env for that determination.
    pub fn new(
        parser_config: ParserConfig,
        options: &'a [ProgramOption],
        env: &'a ParsedEnv,
    ) -> Result<Self, Error> {
        let options = options.iter().collect::<Vec<&'a ProgramOption>>();

        // Build a clap command
        let mut command = Command::new(parser_config.name);

        // Apply settings from parser_config
        if let Some(about) = parser_config.about.as_ref() {
            command = command.about(&**about);
        }

        command = command.args(
            options
                .iter()
                .map(|opt| Self::make_arg(&parser_config, env, opt)),
        );

        if parser_config.no_help_flag {
            command = command.disable_help_flag(true);
        }

        command.build();

        Ok(Self {
            parser_config,
            options,
            env,
            command,
        })
    }

    /// Parse from raw os args (or something that looks like std::env::args_os but could be test data)
    pub fn parse<T>(&self, args_os: impl IntoIterator<Item = T>) -> Result<ArgMatches, Error>
    where
        T: Into<OsString> + Clone,
    {
        self.command.clone().try_get_matches_from(args_os)
    }

    // Turn a ProgramOption into an arg
    fn make_arg(_parser_config: &ParserConfig, env: &ParsedEnv, option: &'a ProgramOption) -> Arg {
        let mut arg = Arg::new(option.id.clone().into_owned()).required(option.is_required);

        // Set the short form if present
        if let Some(short_form) = option.short_form {
            arg = arg.short(short_form);
        }

        // Set the long form if present
        if let Some(long_form) = option.long_form.as_ref() {
            arg = arg.long(long_form.clone().into_owned());
        }

        // Set the env form if present
        if let Some(env_form) = option.env_form.as_ref() {
            let env = env.clone();
            let env_source = move |var: &std::ffi::OsStr| {
                var.to_str().and_then(|var_str| env.get(var_str).cloned())
            };
            arg = arg.env_with_source(env_form.clone().into_owned(), env_source);
        }

        // Set the default value if present
        if let Some(default_value) = option.default_value.as_ref() {
            arg = arg.default_value(default_value.clone().into_owned());
        }

        // Set the help text if present
        if let Some(description) = option.description.as_ref() {
            arg = arg.help(description.clone().into_owned());
        }

        // Set the ArgAction of the arg based on its parse type
        // We typically allow hyphen values.
        match option.parse_type {
            ParseType::Flag => {
                // See also https://github.com/clap-rs/clap/issues/1649
                arg = arg
                    .action(ArgAction::SetTrue)
                    .value_parser(clap::builder::FalseyValueParser::new())
                //arg = arg.action(ArgAction::Set).default_value("false").value_parser(clap::builder::FalseyValueParser::new())
            }
            ParseType::Parameter => arg = arg.action(ArgAction::Set).allow_hyphen_values(true),
            ParseType::Repeat => arg = arg.action(ArgAction::Append).allow_hyphen_values(true),
        };

        // Set the help heading.
        // If there is a short-form or long-form set, then it is listed as a "flag" or "option" as is traditional.
        // If there is only env set, then it is listed as "environment variable".
        // TODO: Needs more work
        /*
        if option.short_form.is_some() || option.long_form.is_some() {
            match option.parse_type {
                ParseType::Flag => {
                    arg = arg.help_heading("Flags");
                }
                ParseType::Parameter | ParseType::Repeat => {
                    arg = arg.help_heading("Options");
                }
            }
        } else {
            arg = arg.help_heading("Environment Variables");
        }*/

        arg
    }

    // This function is not used in the actual crate, since clap handles all the help stuff, but it's here and marked public for testing
    #[doc(hidden)]
    pub fn render_clap_help(&self) -> String {
        let mut command = self.command.clone();
        command.set_bin_name("."); // Override the crate name stuff for tests
        command.render_help().to_string()
    }
}
