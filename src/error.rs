use crate::{ConfValueSource, FlattenedOptionalDebugInfo, ProgramOption};
use clap::{builder::Styles, error::ErrorKind, Command, Error as ClapError};
use std::{ffi::OsString, fmt, fmt::Write};

/// An error which occurs when a `Conf::parse` function is called.
/// This may conceptually represent many underlying errors of several different types.
//
// Note: For now this is a thin wrapper around clap::Error just so that we can control our public
// API independently of clap. We may eventually need to make it contain an enum which is either a
// clap::Error or a collection of InnerError or something like this, but this approach is working
// adequately for now.
#[derive(Debug)]
pub struct Error(ClapError);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error {
    /// Print formatted and colored error text to stderr or stdout as appropriate (as clap does)
    pub fn print(&self) -> Result<(), std::io::Error> {
        self.0.print()
    }

    /// Exit the program, printing an error message to stderr or stdout as appropriate (as clap
    /// does)
    pub fn exit(&self) -> ! {
        self.0.exit()
    }

    /// The exit code this error will exit the program with
    pub fn exit_code(&self) -> i32 {
        self.0.exit_code()
    }

    // An error reported during program options generation
    #[doc(hidden)]
    pub fn skip_short_not_found(
        not_found_chars: Vec<char>,
        field_name: &'static str,
        field_type_name: &'static str,
    ) -> Self {
        let buf = format!("Internal error (invalid skip short)\n  When flattening {field_type_name} at {field_name}, these short options were not found: {not_found_chars:?}\n  To fix this error, remove them from the skip_short attribute list.");
        ClapError::raw(ErrorKind::UnknownArgument, buf).into()
    }
}

impl From<ClapError> for Error {
    fn from(src: ClapError) -> Error {
        Error(src)
    }
}

impl From<fmt::Error> for Error {
    fn from(src: fmt::Error) -> Error {
        ClapError::from(src).into()
    }
}

/// A single problem that occurs when a Conf attempts to parse env, or run a value parser, or run a
/// validation predicate
#[doc(hidden)]
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum InnerError {
    /// Missing a required parameter
    // (missing program option, optional reason it is required)
    MissingRequiredParameter(
        Box<ProgramOption>,
        Option<Box<OwnedFlattenedOptionalDebugInfo>>,
    ),
    /// Invalid parameter value
    // (source, value string, program option, error message)
    InvalidParameterValue(ConfValueSource<String>, String, Box<ProgramOption>, String),
    /// Too Few Arguments
    // (struct name, instance id prefix, single options, flattened fields, optional reason this is
    // required)
    TooFewArguments(
        String,
        String,
        Vec<ProgramOption>,
        Vec<String>,
        Option<Box<OwnedFlattenedOptionalDebugInfo>>,
    ),
    /// Too many arguments
    // (struct name, instance id prefix, single options, flattened fields (field name, option which
    // appeared))
    TooManyArguments(
        String,
        String,
        Vec<(ProgramOption, ConfValueSource<String>)>,
        Vec<(String, ProgramOption, ConfValueSource<String>)>,
    ),
    /// Validation failed
    // (struct name, instance id_prefix, error message)
    ValidationFailed(String, String, String),
    /// Invalid UTF-8 in env (value omitted if it is secret)
    InvalidUtf8Env(String, Box<ProgramOption>, Option<OsString>),
}

impl InnerError {
    /// Helper which makes InvalidParameterValue
    pub fn invalid_value(
        conf_value_source: ConfValueSource<&str>,
        value_str: &str,
        program_option: &ProgramOption,
        err: impl fmt::Display,
    ) -> Self {
        let program_option = Box::new(program_option.clone());
        Self::InvalidParameterValue(
            conf_value_source.into_owned(),
            value_str.to_owned(),
            program_option,
            err.to_string(),
        )
    }

    /// Helper which makes MissingRequiredParameter
    pub(crate) fn missing_required_parameter(
        opt: &ProgramOption,
        flattened_optional_debug_info: Option<FlattenedOptionalDebugInfo<'_>>,
    ) -> Self {
        Self::MissingRequiredParameter(
            Box::new(opt.clone()),
            flattened_optional_debug_info.map(Into::into).map(Box::new),
        )
    }

    /// Helper which makes TooFewArguments
    pub(crate) fn too_few_arguments<'a>(
        struct_name: &'static str,
        instance_id_prefix: &str,
        constraint_single_options: impl AsRef<[&'a ProgramOption]>,
        constraint_flattened_ids: impl AsRef<[&'a str]>,
        flattened_optional_debug_info: Option<FlattenedOptionalDebugInfo<'a>>,
    ) -> Self {
        let constraint_single_options = constraint_single_options
            .as_ref()
            .iter()
            .map(|opt| (*opt).clone())
            .collect::<Vec<_>>();
        let constraint_flattened_ids = constraint_flattened_ids
            .as_ref()
            .iter()
            .map(|id| (*id).to_owned())
            .collect::<Vec<_>>();
        let flattened_optional_debug_info =
            flattened_optional_debug_info.map(Into::into).map(Box::new);
        Self::TooFewArguments(
            struct_name.to_owned(),
            instance_id_prefix.to_owned(),
            constraint_single_options,
            constraint_flattened_ids,
            flattened_optional_debug_info,
        )
    }

    /// Helper which makes TooManyArguments
    // Constraint flattened data is (field name, option which appeared)
    pub(crate) fn too_many_arguments<'a>(
        struct_name: &'static str,
        instance_id_prefix: &'a str,
        constraint_single_options: impl AsRef<[(&'a ProgramOption, ConfValueSource<&'a str>)]>,
        constraint_flattened_data: impl AsRef<[(&'a str, &'a ProgramOption, ConfValueSource<&'a str>)]>,
    ) -> Self {
        let constraint_single_options = constraint_single_options
            .as_ref()
            .iter()
            .map(|(opt, src)| ((*opt).clone(), src.clone().into_owned()))
            .collect::<Vec<_>>();
        let constraint_flattened_data = constraint_flattened_data
            .as_ref()
            .iter()
            .map(|(field_name, opt, src)| {
                (
                    (*field_name).to_owned(),
                    (*opt).clone(),
                    src.clone().into_owned(),
                )
            })
            .collect::<Vec<_>>();
        Self::TooManyArguments(
            struct_name.to_owned(),
            instance_id_prefix.to_owned(),
            constraint_single_options,
            constraint_flattened_data,
        )
    }

    /// Helper which makes ValidationFailed
    pub fn validation(
        struct_name: &'static str,
        instance_id_prefix: &str,
        err: impl fmt::Display,
    ) -> Self {
        Self::ValidationFailed(
            struct_name.to_owned(),
            instance_id_prefix.to_owned(),
            err.to_string(),
        )
    }

    /// Helper which makes InvalidUtf8Env
    pub(crate) fn invalid_utf8_env(
        env_var: &str,
        program_option: &ProgramOption,
        val: Option<&OsString>,
    ) -> Self {
        Self::InvalidUtf8Env(
            env_var.to_owned(),
            Box::new(program_option.clone()),
            val.cloned(),
        )
    }

    // A short (one-line) description of the problem
    fn title(&self) -> &'static str {
        match self {
            Self::InvalidUtf8Env(..) => "An env var contained invalid UTF8",
            Self::MissingRequiredParameter(..) => "A required value was not provided",
            Self::TooFewArguments(..) => "Too few arguments",
            Self::TooManyArguments(..) => "Too many arguments",
            Self::ValidationFailed(..) => "Validation failed",
            Self::InvalidParameterValue(..) => "Invalid value",
        }
    }

    // convert to clap error kind
    fn error_kind(&self) -> ErrorKind {
        match self {
            Self::InvalidUtf8Env(..) => ErrorKind::InvalidValue,
            Self::MissingRequiredParameter(..) => ErrorKind::MissingRequiredArgument,
            Self::TooFewArguments(..) => ErrorKind::TooFewValues,
            Self::TooManyArguments(..) => ErrorKind::TooManyValues,
            Self::ValidationFailed(..) => ErrorKind::ValueValidation,
            Self::InvalidParameterValue(..) => ErrorKind::InvalidValue,
        }
    }

    // get program option associated to this error, if any
    // when only one error occurs, we print associated help text
    fn get_program_option(&self) -> Option<&ProgramOption> {
        match self {
            Self::InvalidUtf8Env(_, opt, _) => Some(opt),
            Self::MissingRequiredParameter(opt, ..) => Some(opt),
            Self::InvalidParameterValue(_src, _val_str, opt, _err) => Some(opt),
            Self::TooFewArguments(..) => None,
            Self::TooManyArguments(..) => None,
            Self::ValidationFailed(..) => None,
        }
    }

    // Print this error in a form for when it is the only error
    fn print_solo(
        &self,
        stream: &mut impl std::fmt::Write,
        styles: &Styles,
    ) -> Result<(), std::fmt::Error> {
        writeln!(stream, "{}", self.title())?;

        self.print_body(stream, styles)?;

        // Print the opt.print help text as well
        if let Some(opt) = self.get_program_option() {
            writeln!(stream)?;
            writeln!(stream, "Help:")?;
            opt.print(stream, None)?;
        }

        Ok(())
    }

    // Print indented details of this error (but not the opt.print text)
    fn print_body(
        &self,
        stream: &mut impl std::fmt::Write,
        styles: &Styles,
    ) -> Result<(), std::fmt::Error> {
        // Styling based on examples in clap like here:
        // https://docs.rs/clap_builder/4.5.9/src/clap_builder/error/mod.rs.html#790
        let invalid = styles.get_invalid();

        match self {
            Self::InvalidUtf8Env(name, opt, maybe_val) => {
                let lossy_val = maybe_val
                    .as_ref()
                    .and_then(|val| {
                        if opt.is_secret() {
                            None
                        } else {
                            Some(val.to_string_lossy())
                        }
                    })
                    .unwrap_or_else(|| "***secret***".into());

                writeln!(
                    stream,
                    "  {name}: {}'{lossy_val}'{}",
                    invalid.render(),
                    invalid.render_reset()
                )?;
            }
            Self::MissingRequiredParameter(opt, maybe_flatten_optional_debug_info) => {
                print_opt_requirements(stream, opt, "must be provided")?;
                if let Some(flatten_optional) = maybe_flatten_optional_debug_info.as_ref() {
                    // Indent 4 spaces
                    write!(stream, "    ")?;
                    flatten_optional.print_required_opt_context(stream)?;
                }
            }
            Self::InvalidParameterValue(value_source, value_str, opt, err) => {
                let context = format!(
                    "  when parsing {} value",
                    render_provided_opt(opt, value_source)
                );
                let mut estimated_len = context.len();
                write!(stream, "{context}")?;
                if !opt.is_secret() {
                    write!(
                        stream,
                        " {}'{value_str}'{}",
                        invalid.render(),
                        invalid.render_reset()
                    )?;
                    estimated_len += 3 + value_str.len();
                }
                writeln!(
                    stream,
                    ": {err_str}",
                    err_str = Self::format_err_str(err, estimated_len + 2)
                )?;
            }
            Self::TooFewArguments(
                struct_name,
                instance_id_prefix,
                single_opts,
                flattened_opts,
                maybe_flatten_optional_debug_info,
            ) => {
                let mut instance_id_prefix = instance_id_prefix.to_owned();
                if !instance_id_prefix.is_empty() {
                    instance_id_prefix.insert_str(0, " @ .");
                    remove_trailing_dot(&mut instance_id_prefix);
                }
                writeln!(stream, "  One of these must be provided: (constraint on {struct_name}{instance_id_prefix}): ")?;
                for opt in single_opts {
                    write!(stream, "  ")?;
                    print_opt_requirements(stream, opt, "")?;
                }
                for field_name in flattened_opts {
                    writeln!(stream, "    Argument group '{field_name}'")?;
                }
                if let Some(flatten_optional) = maybe_flatten_optional_debug_info.as_ref() {
                    write!(stream, "  ")?;
                    flatten_optional.print_required_opt_context(stream)?;
                }
            }
            Self::TooManyArguments(
                struct_name,
                instance_id_prefix,
                single_opts,
                flattened_opts,
            ) => {
                let mut instance_id_prefix = instance_id_prefix.to_owned();
                if !instance_id_prefix.is_empty() {
                    instance_id_prefix.insert_str(0, " @ .");
                    remove_trailing_dot(&mut instance_id_prefix);
                }
                writeln!(stream, "  Too many arguments, provide at most one of these: (constraint on {struct_name}{instance_id_prefix}): ")?;
                for (opt, source) in single_opts {
                    let provided_opt = render_provided_opt(opt, source);
                    writeln!(stream, "    {provided_opt}")?;
                }
                for (field_name, opt, source) in flattened_opts {
                    let provided_opt = render_provided_opt(opt, source);
                    writeln!(
                        stream,
                        "    {provided_opt} (part of argument group '{field_name}')"
                    )?;
                }
            }

            Self::ValidationFailed(struct_name, instance_id_prefix, err) => {
                let mut context = format!("  {struct_name} value was invalid");
                if !instance_id_prefix.is_empty() {
                    context += &format!(" (@ .{instance_id_prefix})");
                }
                let estimated_len = context.len();
                writeln!(
                    stream,
                    "{context}: {err_str}",
                    err_str = Self::format_err_str(err, estimated_len + 2)
                )?;
            }
        }
        Ok(())
    }

    // Formats an error string to look nicely indented if it has line breaks and starting with a
    // line break if the error is long.
    fn format_err_str(err_str: &str, estimated_line_length_so_far: usize) -> String {
        const TARGET_LINE_LENGTH: usize = 100;
        const INDENTATION: usize = 4;

        let mut err_str = err_str.to_owned();
        if err_str.len() + estimated_line_length_so_far > TARGET_LINE_LENGTH
            || err_str.contains('\n')
        {
            err_str.insert(0, '\n');
        }

        let indented_newline = "\n".to_owned() + &" ".repeat(INDENTATION);
        err_str.replace('\n', &indented_newline)
    }
}

// Print one line describing a missing required option, possibly with some additional context
// This is indented two spaces
fn print_opt_requirements(
    stream: &mut impl std::fmt::Write,
    opt: &ProgramOption,
    trailing_text: &str,
) -> fmt::Result {
    let maybe_switch = render_help_switch(opt);
    match (maybe_switch, opt.env_form.as_deref()) {
        (Some(switch), Some(name)) => {
            let trailing_text = if trailing_text.is_empty() {
                "".to_owned()
            } else {
                ", ".to_owned() + trailing_text
            };
            writeln!(stream, "  env '{name}', or '{switch}'{trailing_text}")?
        }
        (Some(switch), None) => writeln!(stream, "  '{switch}' {trailing_text}")?,
        (None, Some(name)) => writeln!(stream, "  env '{name}' {trailing_text}")?,
        (None, None) => {
            debug_assert!(false, "This should be unreachable, we should not be printing opt requirements for an option with no way to specify it");
            writeln!(
                stream,
                "  There is no way to provide this value, this is an internal error ({id})",
                id = opt.id
            )?;
        }
    };
    Ok(())
}

fn render_provided_opt(opt: &ProgramOption, value_source: &ConfValueSource<String>) -> String {
    match value_source {
        ConfValueSource::Args => {
            // If we have both a long and a short form, prefer to display the long form in this help
            // message
            let switch = render_help_switch(opt).unwrap_or_default();
            format!("'{switch}'")
        }
        ConfValueSource::Default => "default value".into(),
        ConfValueSource::Env(name) => {
            format!("env '{name}'")
        }
    }
}

fn render_help_switch(opt: &ProgramOption) -> Option<String> {
    // If we have both a long and a short form, prefer to display the long form in this help message
    opt.long_form
        .as_deref()
        .map(|l| format!("--{l}"))
        .or_else(|| opt.short_form.map(|s| format!("-{s}")))
}

fn remove_trailing_dot(string: &mut String) {
    if let Some(c) = string.pop() {
        if c != '.' {
            string.push(c);
        }
    }
}

/// A version of FlattenedOptionalDebugInfo that owns its data
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct OwnedFlattenedOptionalDebugInfo {
    pub struct_name: &'static str,
    pub id_prefix: String,
    pub option_appeared: Box<ProgramOption>,
    pub value_source: ConfValueSource<String>,
}

impl<'a> From<FlattenedOptionalDebugInfo<'a>> for OwnedFlattenedOptionalDebugInfo {
    fn from(src: FlattenedOptionalDebugInfo<'a>) -> Self {
        Self {
            struct_name: src.struct_name,
            id_prefix: src.id_prefix,
            option_appeared: Box::new(src.option_appeared.clone()),
            value_source: src.value_source.into_owned(),
        }
    }
}

impl OwnedFlattenedOptionalDebugInfo {
    /// Print context about why an option is required, if it is part of an flatten optional group.
    /// Prints one line with no indentation, add indentation first if needed
    fn print_required_opt_context(&self, stream: &mut impl std::fmt::Write) -> fmt::Result {
        let provided_opt = render_provided_opt(&self.option_appeared, &self.value_source);
        let mut context = self.struct_name.to_owned();
        if !self.id_prefix.is_empty() {
            context += " @ .";
            context += &self.id_prefix;
            remove_trailing_dot(&mut context);
        }

        writeln!(
            stream,
            "because {provided_opt} was provided (enabling argument group {context})"
        )
    }
}

impl fmt::Display for InnerError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.title())
    }
}

impl InnerError {
    pub(crate) fn into_clap_error(self, command: &Command) -> Error {
        let mut buf = String::new();

        self.print_solo(&mut buf, command.get_styles()).unwrap();
        ClapError::raw(self.error_kind(), buf)
            .with_cmd(command)
            .into()
    }

    pub(crate) fn vec_to_clap_error(mut src: Vec<InnerError>, command: &Command) -> Error {
        assert!(!src.is_empty());
        if src.len() == 1 {
            return src.remove(0).into_clap_error(command);
        }

        src.sort();
        let last_error_kind = src.last().unwrap().error_kind();

        let styles = command.get_styles();
        let error_sty = styles.get_error();

        let mut buf = String::new();

        let mut last_title = "";
        for err in src {
            if err.title() != last_title {
                if !last_title.is_empty() {
                    write!(
                        &mut buf,
                        "{}error: {}",
                        error_sty.render(),
                        error_sty.render_reset()
                    )
                    .unwrap();
                }
                writeln!(&mut buf, "{}", err.title()).unwrap();
                last_title = err.title();
            }
            err.print_body(&mut buf, styles).unwrap();
        }
        ClapError::raw(last_error_kind, buf)
            .with_cmd(command)
            .into()
    }
}
