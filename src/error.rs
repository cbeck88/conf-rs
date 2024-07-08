use clap::{error::ErrorKind, parser::ValueSource};
use std::fmt;

pub use clap::Error;

/// A single problem that occurs when a Conf attempts to parse CLI args or env
#[doc(hidden)]
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum InnerError {
    /// Missing a required parameter
    // Note: this error should normally be unreachable, we should usually hit MissingRequired in the parser instead.
    // This one is used if at the time that we try to map the match to the user-specified type, we can't find it.
    MissingRequiredParameter(Option<char>, Option<&'static str>, Option<&'static str>),
    /// Invalid parameter value
    InvalidParameterValue(ValueSource, String),
}

impl InnerError {
    // A short (one-line) description of the problem
    fn title(&self) -> impl fmt::Display {
        match self {
            Self::MissingRequiredParameter(..) => "A required value was not provided",
            Self::InvalidParameterValue(..) => "Invalid parameter value",
        }
    }

    fn error_kind(&self) -> ErrorKind {
        match self {
            Self::MissingRequiredParameter(..) => ErrorKind::MissingRequiredArgument,
            Self::InvalidParameterValue(..) => ErrorKind::InvalidValue,
        }
    }

    fn print(&self, stream: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        writeln!(stream, "{}", self.title())?;

        match self {
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
                writeln!(stream, "  when parsing value assigned to {src:?}: {err}")?;
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

// Conversions from InnerError to clap::Error
impl From<InnerError> for Error {
    fn from(src: InnerError) -> Error {
        let mut buf = Vec::<u8>::new();

        src.print(&mut buf).unwrap();

        let message = std::str::from_utf8(&buf).unwrap();
        Error::raw(src.error_kind(), message)
    }
}
