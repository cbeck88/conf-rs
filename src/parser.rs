use crate::{Error, ParseType, ParsedEnv, ProgramOption};
use clap::{Arg, ArgAction, ArgMatches, Command, builder::ValueParser};
use std::{collections::HashMap, ffi::OsString};

/// Result of parsing arguments
#[derive(Clone)]
pub struct ParsedArgs<'a> {
    // Clap's results of parsing the CLI arguments
    pub arg_matches: &'a ArgMatches,
    // A reference to the parser that produced these arg matches.
    // This is needed to:
    // * Access the program options
    // * When checking for subcommands, be able to update to the subcommand parser, so that the
    //   correct program options are used.
    pub parser: &'a Parser,
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

    /// Get a program option by its ID
    pub fn get_program_option(&self, id: &str) -> Option<&'a ProgramOption> {
        self.parser
            .id_to_option
            .get(id)
            .map(|&idx| &self.parser.options[idx])
    }

    /// Get all available program option IDs (for debugging)
    #[doc(hidden)]
    pub fn get_available_ids(&self) -> Vec<&str> {
        self.parser
            .id_to_option
            .keys()
            .map(|s| s.as_str())
            .collect()
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
    /// Optional terminal styling for help text
    pub styles: Option<crate::Styles>,
}

/// A parser which tries to parse args, matching them to a list of ProgramOptions.
#[derive(Clone)]
pub struct Parser {
    options: Vec<ProgramOption>,
    id_to_option: HashMap<String, usize>,
    subcommands: Vec<Parser>,
    command: Command,
}

// Build help text for a ProgramOption, including env vars, defaults, and secret tags.
// The description (if any) comes first, followed by env vars, default value, and secret tag.
fn build_help_text(option: &ProgramOption, env: &ParsedEnv) -> String {
    let mut help_text = option
        .description
        .as_deref()
        .unwrap_or_default()
        .to_string();

    // Add env var info
    if let Some(env_form) = option.env_form.as_deref() {
        let cur_val = env.get_lossy_or_default(env_form);
        help_text += &format!("\n[env {env_form}={cur_val}]");
    }
    for env_alias in option.env_aliases.iter() {
        let cur_val = env.get_lossy_or_default(env_alias);
        help_text += &format!("\n[env {env_alias}={cur_val}]");
    }

    // Add default value
    if let Some(fmt_fn) = option.default_help_str {
        use core::fmt::Write;
        help_text += "\n[default: ";
        write!(&mut help_text, "{}", fmt_fn).unwrap_or_else(|_| {
            panic!(
                "default_help_str function for option '{}' returned an error",
                option.id
            )
        });
        help_text += "]";
    }

    // Add secret tag
    if option.is_secret() {
        help_text += "\n[secret]";
    }

    help_text
}

impl Parser {
    /// Create a parser from top-level parser config and a list of program options
    /// This parser doesn't consider env at all when parsing, but does use env when rendering help.
    pub fn new(
        parser_config: ParserConfig,
        options: Vec<ProgramOption>,
        subcommands: impl AsRef<[Parser]>,
        env: &ParsedEnv,
    ) -> Result<Self, Error> {
        let subcommands = subcommands.as_ref();
        let id_to_option = options
            .iter()
            .enumerate()
            .map(|(idx, opt)| (opt.id.as_ref().to_owned(), idx))
            .collect::<HashMap<String, usize>>();

        // Build a clap command
        let mut command = Command::new(parser_config.name);

        // Apply settings from parser_config
        if let Some(about) = parser_config.about.as_ref() {
            command = command.about(&**about);
        }
        if let Some(ref styles) = parser_config.styles {
            command = command.styles(styles.clone().into_clap_styles());
        }

        let mut args = Vec::<Arg>::new();
        let mut env_only_help_text = Vec::<String>::new();

        // Collect positional args and assign indices
        let mut positional_index = 1usize;
        let mut positional_indices = HashMap::new();
        let mut last_optional_positional: Option<&str> = None;
        let mut last_repeat_positional: Option<&str> = None;
        for opt in options.iter() {
            if opt.is_positional {
                // Validate: no positional arguments can appear after a repeat positional
                if let Some(last_repeat) = last_repeat_positional {
                    panic!(
                        "Positional argument '{}' cannot come after repeat positional argument '{}'",
                        opt.id, last_repeat
                    );
                }
                // Validate: optional positionals can only appear at the end
                if let Some(last_optional) = last_optional_positional {
                    if opt.is_required {
                        panic!(
                            "Required positional argument '{}' cannot come after optional positional argument '{}'",
                            opt.id, last_optional
                        );
                    }
                }
                if !opt.is_required {
                    last_optional_positional = Some(&opt.id);
                }
                if opt.parse_type == ParseType::Repeat {
                    last_repeat_positional = Some(&opt.id);
                }
                positional_indices.insert(&*opt.id, positional_index);
                positional_index += 1;
            }
        }

        for opt in options.iter() {
            let index = positional_indices.get(&*opt.id).copied();
            match Self::make_arg(&parser_config, env, opt, index)? {
                MaybeArg::Arg(arg) => {
                    args.push(arg);
                }
                MaybeArg::EnvOnly(text) => {
                    env_only_help_text.push(text);
                }
                MaybeArg::NotArgsOrEnv => {
                    // We don't document these since the user can't adjust them via CLI or env
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
            options,
            id_to_option,
            subcommands: subcommands.to_vec(),
            command,
        })
    }

    /// Get a reference to the program options
    pub fn get_program_options(&self) -> &[ProgramOption] {
        &self.options
    }

    /// Rename a parser. (This is used by subcommands)
    pub fn rename(mut self, name: impl Into<String>) -> Self {
        self.command = self.command.name(name.into());
        self
    }

    /// Add an alias to a parser. (This is used by subcommands)
    pub fn add_alias(mut self, alias: impl Into<String>) -> Self {
        self.command = self.command.alias(alias.into());
        self
    }

    /// Get command associated to this parser
    pub fn get_command(&self) -> &Command {
        &self.command
    }

    /// Consume this parser and return the command
    pub fn into_command(self) -> Command {
        self.command
    }

    /// Parse from raw os args (or something that looks like std::env::args_os but could be test
    /// data)
    pub(crate) fn parse<T>(
        &mut self,
        args_os: impl IntoIterator<Item = T>,
    ) -> Result<ArgMatches, Error>
    where
        T: Into<OsString> + Clone,
    {
        Ok(self.command.try_get_matches_from_mut(args_os)?)
    }

    // Turn a ProgramOption into an arg. Or, if it should not be set via CLI at all, just generate
    // help text for it which we will append to the help message.
    //
    // Notes:
    //   Our goal here is to get clap to parse the CLI args, and generate satisfactory help text,
    //   but we don't actually want it to handle env, because it's missing a lot of functionality
    //   around that. So some things that clap has nominal support for, we're not going to build
    //   into the command here.
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
    // arguments to be explicit opt-in.
    //
    // For similar reasons, we can't let clap perform default values for env-only arguments, since
    // it won't run for those arguments. It's simpler to just let not clap perform default
    // values at all.
    #[inline]
    fn make_arg(
        _parser_config: &ParserConfig,
        env: &ParsedEnv,
        option: &ProgramOption,
        positional_index: Option<usize>,
    ) -> Result<MaybeArg, Error> {
        // Handle positional arguments
        if option.is_positional {
            debug_assert!(matches!(
                option.parse_type,
                ParseType::Parameter | ParseType::Repeat
            ));
            debug_assert!(option.short_form.is_none());
            debug_assert!(option.long_form.is_none());
            debug_assert!(positional_index.is_some());

            let action = match option.parse_type {
                ParseType::Parameter => ArgAction::Set,
                ParseType::Repeat => ArgAction::Append,
                _ => unreachable!(),
            };

            let mut arg = Arg::new(option.id.clone().into_owned())
                .index(positional_index.unwrap())
                .required(false) // All args are optional from clap's view, we check requirements later
                .action(action)
                .allow_hyphen_values(option.allow_hyphen_values)
                .value_parser(ValueParser::os_string());

            // For repeat positionals, allow multiple values
            if option.parse_type == ParseType::Repeat {
                arg = arg.num_args(1..);
            }

            // Build help text (env is allowed for positional args!)
            let help_text = build_help_text(option, env);
            if !help_text.is_empty() {
                arg = arg.help(help_text);
            }

            return Ok(MaybeArg::Arg(arg));
        }

        if option.short_form.is_none() && option.long_form.is_none() {
            // If there is no short form and no long form, clap is going to make it a positional
            // argument, but we don't want that and there's no way to disable the behavior.
            // Clap also isn't supposed to read a value for this, so the solution is don't create an
            // arg at all, and just add documentation about it ourselves.
            return if option.env_form.is_some() {
                let mut buf = String::new();
                option.print(&mut buf, Some(env))?;
                Ok(MaybeArg::EnvOnly(buf))
            } else if option.default_help_str.is_some()
                || option.has_serde_source
                || !option.is_required
            {
                // This option is not visible in CLI help since it can only come from
                // default values, serde deserialization, or is optional (defaults to None)
                Ok(MaybeArg::NotArgsOrEnv)
            } else {
                panic!(
                    "Program option {option:#?} has no way to receive a value, this is an internal error."
                );
            };
        }

        if option.is_secret() {
            let mut buf = String::new();
            option.print(&mut buf, Some(env)).unwrap();
            panic!(
                "The secret feature is not compatible with arguments that can be read from CLI args. See documentation for more about this.\n\n{buf}"
            )
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

        // Set the help text
        let help_text = build_help_text(option, env);
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
                    .value_parser(ValueParser::os_string())
            }
            ParseType::Repeat => {
                arg = arg
                    .action(ArgAction::Append)
                    .allow_hyphen_values(option.allow_hyphen_values)
                    .value_parser(ValueParser::os_string())
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
    /// Option has no CLI or env source, only default value or serde deserialization
    NotArgsOrEnv,
}
