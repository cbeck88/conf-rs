//! A `derive`-based, highly composable env-and-argument parser aimed at the practically-minded web
//! developer building large web projects.
//!
//! To use `conf`, use the `#[derive(Conf)]` proc macro on your configuration struct.
//! Then call a [`Conf`] trait function to parse your configuration struct.
//! Proc macro attributes are documented there.
//!
//! See README for an overview.
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
use parse_env::parse_env;
use parser::ParsedArgs;
use str_to_bool::str_to_bool;

// Conf, Subcommands, and perhaps Error, is the only public API, but the derive macro needs these
// other types.
pub use error::Error;
pub use traits::{Conf, Subcommands};

#[doc(hidden)]
pub use conf_context::{ConfContext, ConfValueSource};
#[doc(hidden)]
pub use error::InnerError;
#[doc(hidden)]
pub use parse_env::ParsedEnv;
#[doc(hidden)]
pub use parser::{Parser, ParserConfig};
#[doc(hidden)]
pub use program_option::{ParseType, ProgramOption};

#[doc(hidden)]
pub use conf_derive::{self, *};

// CowStr is used internally mainly because using it allows us to construct ProgramOption in a const
// way from string literals, but also to modify them if they have to be flattened into something.
type CowStr = std::borrow::Cow<'static, str>;
