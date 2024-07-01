use crate::{ProgramOption, ValueSource};
use std::ffi::OsString;
use std::fmt;

#[doc(hidden)]
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SwitchDescription {
    /// The characters of the switch
    pub name: String,
    /// True if the switch is long form (--) or short form (-)
    pub is_long: bool,
}

impl fmt::Display for SwitchDescription {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_long {
            write!(f, "--")?;
        } else {
            write!(f, "-")?;
        }
        write!(f, "{}", self.name)
    }
}

/// A single problem that occurs when a Conf attempts to parse CLI args or env
#[doc(hidden)]
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum InnerError {
    // A switch was declared with two different parse types
    ParseTypeMismatch(SwitchDescription, Box<ProgramOption>, Box<ProgramOption>),
    /// Switch is unknown, perhaps you meant ... ?
    UnknownSwitch(SwitchDescription, Option<SwitchDescription>),
    /// No value was supplied for parameter
    MissingParameterValue(SwitchDescription),
    /// A parameter was specified twice when only expected once
    ParameterSpecifiedTwice(SwitchDescription),
    /// This argument was not expected, expected another switch
    UnexpectedArgument(String),
    /// A required program option was not provided
    MissingRequired(Box<ProgramOption>),
    /// An argument was not valid utf8
    InvalidUtf8Arg(OsString),
    /// An env value was not valid utf8
    InvalidUtf8Env(String),
    /// A program option was set twice
    ProgramOptionSpecifiedTwice(ValueSource, ValueSource),
    /// Missing a required parameter
    // Note: this error should normally be unreachable, we should usually hit MissingRequired in the parser instead.
    // This one is used if at the time that we try to map the match to the user-specified type, we can't find it.
    MissingRequiredParameter(
        Option<&'static str>,
        Option<&'static str>,
        Option<&'static str>,
    ),
    /// Invalid parameter value
    InvalidParameterValue(ValueSource, String),
}

impl InnerError {
    // A short (one-line) description of the problem
    fn title(&self) -> impl fmt::Display {
        match self {
            Self::ParseTypeMismatch(..) => "A switch was declared with two different parse types",
            Self::UnknownSwitch(..) => "Unknown switch",
            Self::MissingParameterValue(..) => "No value was supplied for parameter",
            Self::ParameterSpecifiedTwice(..) => {
                "A parameter was specified twice, but was only expected once"
            }
            Self::UnexpectedArgument(..) => "Argument was not expected",
            Self::MissingRequired(..) => "A required value was not provided",
            Self::InvalidUtf8Arg(..) => "An argument was not valid utf8",
            Self::InvalidUtf8Env(..) => "A required environment variable was not valid utf8",
            Self::ProgramOptionSpecifiedTwice(..) => {
                "A program option was specified twice, but was only expected once"
            }
            Self::MissingRequiredParameter(..) => "A required value was not provided",
            Self::InvalidParameterValue(..) => "Invalid parameter value",
        }
    }

    fn print(&self, stream: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        writeln!(stream, "{}", self.title())?;

        match self {
            Self::ParseTypeMismatch(switch, opt1, opt2) => {
                writeln!(
                    stream,
                    "  '{switch}' cannot be parsed with type {} and with type {}.\n",
                    opt1.parse_type, opt2.parse_type
                )?;
                writeln!(stream, "  Two program options are using this switch:\n")?;
                opt1.print(stream, None)?;
                opt2.print(stream, None)?;
            }
            Self::UnknownSwitch(switch, maybe_suggestion) => {
                if let Some(sug) = maybe_suggestion.as_ref() {
                    writeln!(stream, "  '{switch}' is unknown, perhaps you meant")?;
                    writeln!(stream, "  '{sug}' ?")?;
                } else {
                    writeln!(stream, "  '{switch}' is unknown")?;
                }
            }
            Self::MissingParameterValue(switch) => {
                writeln!(
                    stream,
                    "  When using '{switch}' you must supply a value, for example,"
                )?;
                writeln!(stream, "  {switch} <VALUE>")?;
                writeln!(stream, "  {switch}=<VALUE>")?;
            }
            Self::ParameterSpecifiedTwice(switch) => {
                writeln!(stream, "  '{switch}' can only have one value")?;
            }
            Self::UnexpectedArgument(arg) => {
                writeln!(
                    stream,
                    "  argument {arg:?} was found, but expected another option to appear next"
                )?;
            }
            Self::MissingRequired(opt) => {
                opt.print(stream, None)?;
            }
            Self::InvalidUtf8Arg(os_string) => {
                writeln!(
                    stream,
                    "  could not parse argument: {}",
                    os_string.to_string_lossy()
                )?;
            }
            Self::InvalidUtf8Env(name) => {
                writeln!(
                    stream,
                    "  could not parse environment variable '{name}' as utf8"
                )?;
            }
            Self::ProgramOptionSpecifiedTwice(src1, src2) => {
                writeln!(stream, "  {src1} and {src2} cannot both be set")?;
            }
            Self::MissingRequiredParameter(maybe_short_form, maybe_long_form, maybe_env_form) => {
                // If we have both a long and a short form, prefer to display the long form in this help message
                let maybe_switch = maybe_long_form
                    .map(|l| format!("--{l}"))
                    .or_else(|| maybe_short_form.map(|s| format!("-{s}")));
                match (maybe_switch, maybe_env_form) {
                    (Some(switch), Some(name)) => {
                        writeln!(stream, "  Either option '{switch}', or environment variable '{name}' must be provided")?
                    },
                    (Some(switch), None) => {
                        writeln!(stream, "  Option '{switch}' must be provided")?
                    },
                    (None, Some(name)) => {
                        writeln!(stream, "  Environment variable '{name}' must be provided")?
                    },
                    (None, None) => {
                        writeln!(stream, "  There is no way to provide this value, this is an internal error")?
                    },
                };
            }
            Self::InvalidParameterValue(src, err) => {
                writeln!(stream, "  when parsing value assigned to {src}: {err}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for InnerError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.title())
    }
}

/// An error (or collection of errors) that occurs when parsing CLI args or env into a Conf
#[derive(Debug)]
pub struct Error {
    errors: Vec<InnerError>,
}

impl From<InnerError> for Error {
    fn from(src: InnerError) -> Self {
        Self { errors: vec![src] }
    }
}

impl From<Vec<InnerError>> for Error {
    fn from(src: Vec<InnerError>) -> Self {
        Self { errors: src }
    }
}

impl Error {
    /// Aggregate errors from another Error into self
    pub fn combine(&mut self, other: Error) {
        self.errors.extend(other.errors);
    }

    /// Print a human readable error report to given stream
    #[doc(hidden)]
    pub fn print(&mut self, stream: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        // Order the errors by type, which generally means earlier errors come first, and then they are in alphabetical order of context.
        self.errors.sort();

        for error in &self.errors {
            error.print(stream)?;
            writeln!(stream)?;
        }
        writeln!(stream, "For more information, try `--help`")
    }

    /// Print a human readable error report on stderr and then exit
    pub fn exit(mut self) -> ! {
        let _ = self.print(&mut std::io::stderr());
        std::process::exit(2)
    }
}
