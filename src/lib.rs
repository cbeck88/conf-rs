//! A `derive`-based, highly composable env-and-argument parser aimed at the practically-minded web
//! developer building large web projects.
#![doc = include_str!("../REFERENCE.md")]
#![deny(unsafe_code)]
#![deny(missing_docs)]

mod conf_context;
mod error;
mod parse_env;
mod parser;
mod program_option;
mod str_to_bool;
mod traits;

// These are not needed by users or by generated code
use conf_context::FlattenedOptionalDebugInfo;
use parse_env::{parse_env, ParsedEnv};
use parser::ParsedArgs;
use str_to_bool::str_to_bool;

// Conf, and perhaps Error, is the only public API, but the derive macro needs these other types.
pub use error::Error;
pub use traits::Conf;

#[doc(hidden)]
pub use conf_context::{ConfContext, ConfValueSource};
#[doc(hidden)]
pub use error::InnerError;
#[doc(hidden)]
pub use parser::ParserConfig;
#[doc(hidden)]
pub use program_option::{ParseType, ProgramOption};

// This is used by tests
#[doc(hidden)]
pub use parser::Parser;

#[doc(hidden)]
pub use conf_derive::{self, *};

// CowStr is used internally mainly because using it allows us to construct ProgramOption in a const
// way from string literals, but also to modify them if they have to be flattened into something.
type CowStr = std::borrow::Cow<'static, str>;
