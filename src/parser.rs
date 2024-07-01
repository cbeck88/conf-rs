use crate::{
    const_str, str_to_bool, CowStr, Error, InnerError, ParseType, ParsedEnv, ProgramOption,
    SwitchDescription,
};
use edit_distance::edit_distance;
use heck::ToShoutySnakeCase;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::Path,
};

/// The result of parsing CLI args based on a parser config and list of program options
/// Parsed args can be used with an env to produce a ConfContext.
#[doc(hidden)]
#[derive(Default)]
pub struct ParsedArgs {
    short_flags: BTreeSet<String>,
    long_flags: BTreeSet<String>,
    short_parameters: BTreeMap<String, String>,
    long_parameters: BTreeMap<String, String>,
    repeats: BTreeMap<String, Vec<String>>,
}

impl ParsedArgs {
    pub fn has_short_flag(&self, name: &str) -> bool {
        self.short_flags.contains(name)
    }
    pub fn has_long_flag(&self, name: &str) -> bool {
        self.long_flags.contains(name)
    }
    pub fn get_short_parameter(&self, name: &str) -> Option<&str> {
        self.short_parameters.get(name).map(String::as_str)
    }
    pub fn get_long_parameter(&self, name: &str) -> Option<&str> {
        self.long_parameters.get(name).map(String::as_str)
    }
    pub fn get_repeat(&self, name: &str) -> &[String] {
        self.repeats.get(name).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Top-level parser config
#[derive(Clone, Debug, Default)]
pub struct ParserConfig {
    /// An optional top-level specified about string
    pub about: Option<CowStr>,
    /// True if help flags should not be automatically generated
    pub no_help_flag: bool,
}

// Special options that are synthesized
static HELP_OPTION: ProgramOption = ProgramOption {
    parse_type: ParseType::Flag,
    description: Some(const_str("Print help")),
    short_form: Some(const_str("h")),
    long_form: Some(const_str("help")),
    env_form: None,
    default_value: None,
    is_required: false,
};

// Data about a switch we intend to parse
struct Switch<'a> {
    parse_type: ParseType,
    // Source program option
    program_option: &'a ProgramOption,
}

type SwitchMap<'a> = BTreeMap<&'a str, Switch<'a>>;

/// A parser which tries to parse args, matching them to a list of ProgramOptions.
pub struct Parser<'a> {
    parser_config: ParserConfig,
    options: Vec<&'a ProgramOption>,
    short_switches: SwitchMap<'a>,
    long_switches: SwitchMap<'a>,
    env: &'a ParsedEnv,
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
        // We need to know what switches we are expecting to parse, and whether they are flags, parameters, or repeat,
        // so that we can parse correctly in context.
        let mut short_switches = SwitchMap::default();
        let mut long_switches = SwitchMap::default();

        let mut options = options.iter().collect::<Vec<&'a ProgramOption>>();

        if !parser_config.no_help_flag {
            options.push(&HELP_OPTION);
        }

        for option in &options {
            Self::record_switches(&mut short_switches, &mut long_switches, option)?;
        }

        Ok(Self {
            parser_config,
            options,
            short_switches,
            long_switches,
            env,
        })
    }

    // Update switch maps based on a program option, and detect collisions
    fn record_switches(
        short_switches: &mut SwitchMap<'a>,
        long_switches: &mut SwitchMap<'a>,
        option: &'a ProgramOption,
    ) -> Result<(), InnerError> {
        if let Some(short_form) = option.short_form.as_ref() {
            let switch = short_switches.entry(short_form).or_insert_with(|| Switch {
                parse_type: option.parse_type,
                program_option: option,
            });
            if switch.parse_type != option.parse_type {
                return Err(InnerError::ParseTypeMismatch(
                    SwitchDescription {
                        name: short_form.clone().into_owned(),
                        is_long: false,
                    },
                    Box::new(option.clone()),
                    Box::new(switch.program_option.clone()),
                ));
            }
        }
        if let Some(long_form) = option.long_form.as_ref() {
            let switch = long_switches.entry(long_form).or_insert_with(|| Switch {
                parse_type: option.parse_type,
                program_option: option,
            });
            if switch.parse_type != option.parse_type {
                return Err(InnerError::ParseTypeMismatch(
                    SwitchDescription {
                        name: long_form.clone().into_owned(),
                        is_long: true,
                    },
                    Box::new(option.clone()),
                    Box::new(switch.program_option.clone()),
                ));
            }
        }

        Ok(())
    }

    /// Parse from raw os args (or something that looks like std::env::args_os but could be test data)
    pub fn parse<T>(&self, args_os: impl IntoIterator<Item = T>) -> Result<ParsedArgs, Error>
    where
        T: Into<OsString> + Clone,
    {
        let mut result = ParsedArgs::default();

        let mut iter = args_os.into_iter();

        // Get the first arg, which is generally the executable name and not an actual argument,
        // and is used later in rendering help.
        //
        // If arg0 is missing, that's pretty wierd, but not a good enough reason to return an error,
        // the user may be trying to implement `Default`
        // using `Conf::try_parse_from(Default::default(), Default::default()).unwrap()`
        // or something like this.
        let os_arg0 = iter
            .next()
            .map(Into::into)
            .unwrap_or_else(|| OsString::from("."));

        while let Some(os_arg) = iter.next() {
            let arg = os_arg
                .into()
                .into_string()
                .map_err(InnerError::InvalidUtf8Arg)?;
            if let Some(rem) = arg.strip_prefix("--") {
                self.parse_switch(rem, true, &mut iter, &mut result, &os_arg0)?;
            } else if let Some(rem) = arg.strip_prefix('-') {
                self.parse_switch(rem, false, &mut iter, &mut result, &os_arg0)?;
            } else {
                // If we had support for positional arguments or subcommands, we would try to match arg to that here...
                return Err(InnerError::UnexpectedArgument(arg).into());
            }
        }

        // Now, check if every required program option was satisfied, taking into account env also.
        // Collect all errors of this type at once, which helps the user iterating on cloud deployments.
        //
        // Implementation-wise, it's easier to do this now and get all the errors at this point,
        // rather than waiting to actually marshall everything.
        let mut errors = Vec::<InnerError>::new();
        for option in &self.options {
            if option.is_required {
                match option.parse_type {
                    ParseType::Flag => panic!("Flags are never required"),
                    ParseType::Repeat => panic!("Repeat options are never required"),
                    ParseType::Help => panic!("Help options are never required"),
                    ParseType::Parameter => {
                        if let Some(short) = option.short_form.as_ref() {
                            if result.short_parameters.contains_key(short.as_ref()) {
                                continue;
                            }
                        }
                        if let Some(long) = option.long_form.as_ref() {
                            if result.long_parameters.contains_key(long.as_ref()) {
                                continue;
                            }
                        }
                        if let Some(name) = option.env_form.as_ref() {
                            if self.env.contains_key(name.as_ref()) {
                                continue;
                            }
                        }
                        errors.push(InnerError::MissingRequired(Box::new((*option).clone())));
                    }
                }
            }
        }
        if errors.is_empty() {
            Ok(result)
        } else {
            Err(Error::from(errors))
        }
    }

    // Parse a long switch or short switch (after the dashes), trying to lookup if it expects an argument, and adding that arg to ParsedArgs if found
    fn parse_switch<T>(
        &self,
        switch: &str,
        is_long: bool,
        iter: &mut impl Iterator<Item = T>,
        result: &mut ParsedArgs,
        arg0: &OsString,
    ) -> Result<(), InnerError>
    where
        T: Into<OsString> + Clone,
    {
        // Check for value set by = and split if it is present
        let (name, maybe_value) = if let Some((name, value)) = switch.split_once('=') {
            (name, Some(value))
        } else {
            (switch, None)
        };
        let switch_map = if is_long {
            &self.long_switches
        } else {
            &self.short_switches
        };
        // Check if we are expecting a switch of this type with this name
        let switch = switch_map.get(name).ok_or_else(|| {
            self.unknown_switch(SwitchDescription {
                name: name.to_owned(),
                is_long,
            })
        })?;

        // If this switch type doesn't expect any arguments, then we can finish handling it right now,
        // otherwise there is shared code for parsing the next argument that follows.
        match switch.parse_type {
            ParseType::Flag => {
                let flags = if is_long {
                    &mut result.long_flags
                } else {
                    &mut result.short_flags
                };

                if let Some(value) = maybe_value {
                    // If this switch is supposed to be boolean, but we have --switch=val, then try to parse val as a boolean string
                    if str_to_bool(value) {
                        flags.insert(name.to_owned());
                    }
                } else {
                    // This is just the normal switch usage
                    flags.insert(name.to_owned());
                }
                return Ok(());
            }
            ParseType::Help => {
                self.render_help(&mut std::io::stderr(), arg0).unwrap();
                std::process::exit(2);
            }
            _ => {}
        };

        // If it's a parameter or repeat, then we need to have a value associated to this switch. It either is set with = syntax, or it's the next argument.
        let value = if let Some(value) = maybe_value {
            // This is syntax `--name=value`
            value.to_owned()
        } else {
            // Expecting syntax `--name value`
            iter.next()
                .ok_or_else(|| {
                    InnerError::MissingParameterValue(SwitchDescription {
                        name: name.to_owned(),
                        is_long,
                    })
                })?
                .into()
                .into_string()
                .map_err(InnerError::InvalidUtf8Arg)?
        };

        match switch.parse_type {
            ParseType::Flag => {
                unreachable!("Flag should have been earlier matched away");
            }
            ParseType::Help => {
                unreachable!("Help should have been earlier matched away");
            }
            ParseType::Parameter => {
                let parameters = if is_long {
                    &mut result.long_parameters
                } else {
                    &mut result.short_parameters
                };

                if let Some(_prev) = parameters.insert(name.to_owned(), value) {
                    return Err(InnerError::ParameterSpecifiedTwice(SwitchDescription {
                        name: name.to_owned(),
                        is_long,
                    }));
                }
            }
            ParseType::Repeat => {
                let list = result.repeats.entry(name.to_owned()).or_default();
                list.push(value);
            }
        };
        Ok(())
    }

    // Renders help text to a given stream
    // This is an implementation detail, but it's marked public for testing
    #[doc(hidden)]
    pub fn render_help(
        &self,
        stream: &mut impl std::io::Write,
        arg0: &OsString,
    ) -> Result<(), std::io::Error> {
        if let Some(about) = self.parser_config.about.as_ref() {
            writeln!(stream, "{about}\n")?;
        }
        // Program options that are required and have at least one switch associated to them
        let required: Vec<&'a ProgramOption> = self
            .options
            .iter()
            .cloned()
            .filter(|opt| opt.is_required && (opt.short_form.is_some() || opt.long_form.is_some()))
            .collect();
        // Program options that are required, but have no switches, and so must have env
        let required_env: Vec<&'a ProgramOption> = self
            .options
            .iter()
            .cloned()
            .filter(|opt| opt.is_required && opt.short_form.is_none() && opt.long_form.is_none())
            .collect();
        let flags: Vec<&'a ProgramOption> = self
            .options
            .iter()
            .cloned()
            .filter(|opt| !opt.is_required && opt.parse_type == ParseType::Flag)
            .collect();
        let optional: Vec<&'a ProgramOption> = self
            .options
            .iter()
            .cloned()
            .filter(|opt| !opt.is_required && opt.parse_type != ParseType::Flag)
            .collect();
        let arg0_str = Self::format_bin_name(arg0).unwrap_or(".");

        write!(stream, "Usage: {arg0_str}")?;

        if !flags.is_empty() {
            write!(stream, " [FLAGS]")?;
        }

        if !optional.is_empty() {
            write!(stream, " [OPTIONS]")?;
        }

        for req in &required {
            if let Some(long) = req.long_form.as_ref() {
                write!(
                    stream,
                    " --{long} <{snake_long}>",
                    snake_long = long.to_shouty_snake_case()
                )?;
            } else if let Some(short) = req.short_form.as_ref() {
                write!(
                    stream,
                    " -{short} <{snake_short}>",
                    snake_short = short.to_shouty_snake_case()
                )?;
            }
        }
        writeln!(stream)?;

        if !(required.is_empty() && optional.is_empty()) {
            writeln!(stream, "\nOptions:")?;
            for req in required {
                req.print(stream, Some(self.env))?;
            }
            for opt in optional {
                opt.print(stream, Some(self.env))?;
            }
        }

        if !flags.is_empty() {
            writeln!(stream, "\nFlags:")?;
            for flag in flags {
                flag.print(stream, Some(self.env))?;
            }
        }

        if !required_env.is_empty() {
            writeln!(stream, "\nRequired env:")?;
            for req_env in required_env {
                req_env.print(stream, Some(self.env))?;
            }
        }

        Ok(())
    }

    fn format_bin_name(arg0: &OsString) -> Option<&str> {
        let p = Path::new(arg0);

        if let Some(f) = p.file_name() {
            return f.to_str();
        }

        None
    }

    // Helper which, when an unknown switch error occurs, tries to suggest a correct switch
    fn unknown_switch(&self, unknown: SwitchDescription) -> InnerError {
        let best_so_far: Option<(SwitchDescription, usize)> = None;

        // Don't bother with scores > 4, also if the unknown switch is very short then the limit should be reduced
        let limit = core::cmp::min(4, unknown.name.len() / 2);

        let best_so_far =
            self.short_switches
                .keys()
                .fold(best_so_far, |best_so_far, short_switch| {
                    // Add a penalty of one to the score if the unknown switch is a -- switch
                    let score = edit_distance(&unknown.name, short_switch)
                        + if unknown.is_long { 1 } else { 0 };
                    if score > limit {
                        best_so_far
                    } else if let Some((_prev_best, prev_score)) = best_so_far.as_ref() {
                        if score < *prev_score {
                            Some((
                                SwitchDescription {
                                    name: (*short_switch).to_owned(),
                                    is_long: false,
                                },
                                score,
                            ))
                        } else {
                            best_so_far
                        }
                    } else {
                        best_so_far
                    }
                });

        let best_so_far =
            self.long_switches
                .keys()
                .fold(best_so_far, |best_so_far, long_switch| {
                    // Add a penalty of one to the score if the unknown switch is a - switch
                    let score = edit_distance(&unknown.name, long_switch)
                        + if unknown.is_long { 0 } else { 1 };
                    if score > limit {
                        best_so_far
                    } else if let Some((_prev_best, prev_score)) = best_so_far.as_ref() {
                        if score < *prev_score {
                            Some((
                                SwitchDescription {
                                    name: (*long_switch).to_owned(),
                                    is_long: true,
                                },
                                score,
                            ))
                        } else {
                            best_so_far
                        }
                    } else {
                        best_so_far
                    }
                });

        InnerError::UnknownSwitch(unknown, best_so_far.map(|pair| pair.0))
    }
}
