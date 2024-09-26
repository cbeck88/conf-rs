use crate::{Error, ParseType, ParsedEnv, ProgramOption};
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{collections::HashMap, ffi::OsString};

/// Result of parsing arguments
#[derive(Clone)]
pub struct ParsedArgs<'a> {
    // Clap's results of parsing the CLI arguments
    pub arg_matches: &'a ArgMatches,
    // A reference to the parser that produced these arg matches.
    // This is needed to:
    // * Keep track of id_to_option map
    // * When checking for subcommands, be able to update to the subcommand parser, so that the
    //   correct id_to_option map is used.
    pub parser: &'a Parser<'a>,
}

impl<'a> ParsedArgs<'a> {
    // Make a ParsedArgs from an ArgMatches and a Parser.
    // This is kind of janky from an API point of view. It would be simpler if calling
    // `Parser::parse` could just return ParsedArgs, with all the needed data, and all the
    // lifetimes would just work. Instead we have to do this two-step thing where we call
    // Parser::parse, and then ParsedArgs::new, because we would need ParsedArgs to have an
    // "owning ref" somehow but that is very difficult in rust unfortunately. Since it's just
    // internal API I don't really care.
    pub(crate) fn new(arg_matches: &'a ArgMatches, parser: &'a Parser) -> Self {
        Self {
            arg_matches,
            parser,
        }
    }

    // Get the id_to_option map from the parser.
    // This is very helpful to the ConfContext to handle env parsing and error reporting.
    pub fn id_to_option(&self) -> &'a HashMap<&'a str, &'a ProgramOption> {
        &self.parser.id_to_option
    }

    // Check if Clap found a subcommand among these matches.
    // If so, return the name, and a correct ParsedArgs for the subcommand, with the subcommand arg
    // matches and reference to the subcommand parser.
    pub fn get_subcommand(&self) -> Option<(String, Self)> {
        self.arg_matches.subcommand().map(|(name, arg_matches)| {
            let parser = self.parser.subcommands.iter().find(|parser| parser.command.get_name() == name).unwrap_or_else(|| {
                let names: Vec<_> = self.parser.subcommands.iter().map(|parser| parser.command.get_name()).collect();
                panic!("Could not find parser matching to subcommand {name}. This is an internal error. Found subcommand names: {:?}", names);
            });

            (
                name.to_owned(),
                Self {
                    arg_matches,
                    parser,
                },
            )
        })
    }
}

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
#[derive(Clone)]
pub struct Parser<'a> {
    #[allow(unused)]
    parser_config: ParserConfig,
    #[allow(unused)]
    options: Vec<&'a ProgramOption>,
    id_to_option: HashMap<&'a str, &'a ProgramOption>,
    subcommands: Vec<Parser<'a>>,
    #[allow(unused)]
    env: &'a ParsedEnv,
    command: Command,
}

impl<'a> Parser<'a> {
    /// Create a parser from top-level parser config and a list of program options
    /// This parser doesn't consider env at all when parsing, but does use env when rendering help.
    pub fn new(
        parser_config: ParserConfig,
        options: &'a [ProgramOption],
        subcommands: impl AsRef<[Parser<'a>]>,
        env: &'a ParsedEnv,
    ) -> Result<Self, Error> {
        let options = options.iter().collect::<Vec<&'a ProgramOption>>();
        let subcommands = subcommands.as_ref();
        let id_to_option = options
            .iter()
            .map(|opt| (&*opt.id, *opt))
            .collect::<HashMap<&'a str, &'a ProgramOption>>();

        // Build a clap command
        let mut command = Command::new(parser_config.name);

        // Apply settings from parser_config
        if let Some(about) = parser_config.about.as_ref() {
            command = command.about(&**about);
        }

        let mut args = Vec::<Arg>::new();
        let mut env_only_help_text = Vec::<String>::new();

        for opt in options.iter() {
            match Self::make_arg(&parser_config, env, opt)? {
                MaybeArg::Arg(arg) => {
                    args.push(arg);
                }
                MaybeArg::EnvOnly(text) => {
                    env_only_help_text.push(text);
                }
                MaybeArg::DefaultOnly => {
                    // We don't bother documenting these since the user can't adjust them
                }
            }
        }

        let subcommand_vec: Vec<_> = subcommands
            .iter()
            .map(|p| p.get_command().clone())
            .collect();
        command = command.args(args).subcommands(subcommand_vec);

        if parser_config.no_help_flag {
            command = command.disable_help_flag(true);
        }

        // Make an environment variables section that goes at the end, in after_help
        if !env_only_help_text.is_empty() {
            let mut after_help_text = "Environment variables:\n".to_owned();

            for var_text in env_only_help_text {
                after_help_text += &var_text;
            }

            command = command.after_help(after_help_text);
        }

        command.build();

        Ok(Self {
            parser_config,
            options,
            id_to_option,
            subcommands: subcommands.to_vec(),
            env,
            command,
        })
    }

    /// Rename a parser. (This is used by subcommands)
    pub fn rename(mut self, name: impl Into<String>) -> Self {
        self.command = self.command.name(name.into());
        self
    }

    /// Get command associated to this parser
    pub fn get_command(&self) -> &Command {
        &self.command
    }

    /// Parse from raw os args (or something that looks like std::env::args_os but could be test
    /// data)
    pub(crate) fn parse<T>(&self, args_os: impl IntoIterator<Item = T>) -> Result<ArgMatches, Error>
    where
        T: Into<OsString> + Clone,
    {
        Ok(self.command.clone().try_get_matches_from(args_os)?)
    }

    // Turn a ProgramOption into an arg. Or, if it should not be set via CLI at all, just generate
    // help text for it which we will append to the help message.
    //
    // Notes:
    //   Our goal here is to get clap to parse the CLI args, and generate satisfactory help text,
    //   but we don't actually want it to handle env, because it's missing a lot of functionality
    // around that.   So some things that clap has nominal support for, we're not going to build
    // into the command here.
    //
    //   Instead, our strategy is:
    //   1. No env is specified to clap. All env handling is going to happen in `ConfContext`
    //      instead.
    //   2. Nothing that is "required" is considered required at this stage, because it might be
    //      supplied by env. We will check for requirements being fulfilled in `ConfContext`
    //      instead, and errors will be aggregated in `from_conf_context`. That also lets us delay
    //      hitting "missing required value" errors until we are prepared to capture all possible
    //      errors that occur.
    //   3. Because no env is specified to clap, it won't show up in the clap generated help text
    //      automatically. Instead we have to put it in the description ourselves.
    //
    // We also need to work around this issue: https://github.com/clap-rs/clap/discussions/5432
    //
    // If an arg doesn't have a short or long flag, then clap will consider it a positional
    // argument. But if it has an env source, it might be a secret or something and it would not
    // be correct to treat it as a positional CLI argument. In this crate we want positional
    // arguments to be opt-in, and we don't support them yet.
    //
    // For similar reasons, we can't let clap perform default values for env-only arguments, since
    // it won't run for those arguments. It's simpler to just let not clap perform default
    // values at all.
    fn make_arg(
        _parser_config: &ParserConfig,
        env: &ParsedEnv,
        option: &'a ProgramOption,
    ) -> Result<MaybeArg, Error> {
        if option.short_form.is_none() && option.long_form.is_none() {
            // If there is no short form and no long form, clap is going to make it a positional
            // argument, but we don't want that and there's no way to disable the behavior.
            // Clap also isn't supposed to read a value for this, so the solution is don't create an
            // arg at all, and just add documentation about it ourselves.
            return if option.env_form.is_some() {
                let mut buf = String::new();
                option.print(&mut buf, Some(env))?;
                Ok(MaybeArg::EnvOnly(buf))
            } else if option.default_value.is_some() {
                Ok(MaybeArg::DefaultOnly)
            } else {
                panic!("Program option {option:#?} has no way to receive a value, this is an internal error.");
            };
        }

        if option.is_secret() {
            let mut buf = String::new();
            option.print(&mut buf, Some(env)).unwrap();
            panic!("The secret feature is not compatible with arguments that can be read from CLI args. See documentation for more about this.\n\n{buf}")
        }

        let mut arg = Arg::new(option.id.clone().into_owned());

        // All args are considered optional from clap's point of view, and we will handle any
        // missing required errors later.
        arg = arg.required(false);

        // Set the short form if present
        if let Some(short_form) = option.short_form {
            arg = arg.short(short_form);
        }

        // Set the long form if present
        if let Some(long_form) = option.long_form.as_ref() {
            arg = arg.long(long_form.clone().into_owned());
        }

        // Set visible aliases if present
        if !option.aliases.is_empty() {
            arg = arg.visible_aliases(
                option
                    .aliases
                    .iter()
                    .map(|alias| alias.clone().into_owned())
                    .collect::<Vec<_>>(),
            );
        }

        // Set the help text if either description or env_form is present, in that order
        let mut help_text = option
            .env_form
            .as_deref()
            .map(|env_form| {
                let cur_val = env.get_lossy_or_default(env_form);
                format!("\n[env {env_form}={cur_val}]")
            })
            .unwrap_or_default();
        // Append any env aliases to the help text
        for env_alias in option.env_aliases.iter() {
            let cur_val = env.get_lossy_or_default(env_alias);
            help_text += &format!("\n[env {env_alias}={cur_val}]");
        }
        // Append any default value to the help text
        if let Some(def) = option.default_value.as_ref() {
            help_text += &format!("\n[default: {def}]");
        }
        // Append secret tag if the option is a secret
        if option.is_secret() {
            help_text += "\n[secret]";
        }
        // Prepend the user's description to the help_text if present
        help_text.insert_str(0, option.description.as_deref().unwrap_or_default());
        if !help_text.is_empty() {
            arg = arg.help(help_text);
        }

        // Set the ArgAction of the arg based on its parse type
        match option.parse_type {
            ParseType::Flag => {
                // See also https://github.com/clap-rs/clap/issues/1649
                arg = arg
                    .action(ArgAction::SetTrue)
                    .value_parser(clap::builder::FalseyValueParser::new())
            }
            ParseType::Parameter => {
                arg = arg
                    .action(ArgAction::Set)
                    .allow_hyphen_values(option.allow_hyphen_values)
            }
            ParseType::Repeat => {
                arg = arg
                    .action(ArgAction::Append)
                    .allow_hyphen_values(option.allow_hyphen_values)
            }
        };

        // Set the help heading.
        // If there is a short-form or long-form set, then it is listed as a "flag" or "option" as
        // is traditional. If there is only env set, then it is listed as "environment
        // variable". TODO: Needs more work
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

        Ok(MaybeArg::Arg(arg))
    }

    // This function is not used in the actual crate, since clap handles all the help stuff, but
    // it's here and marked public for testing
    #[doc(hidden)]
    pub fn render_clap_help(&self) -> String {
        let mut command = self.command.clone();
        command.set_bin_name("."); // Override the crate name stuff for tests
        command.render_help().to_string()
    }
}

#[allow(clippy::large_enum_variant)]
enum MaybeArg {
    Arg(Arg),
    EnvOnly(String),
    DefaultOnly,
}
