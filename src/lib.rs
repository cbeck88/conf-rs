//! A derive-based, highly composable config parsing library with first-class support for env
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
use error::SwitchDescription;
use parse_env::{parse_env, ParsedEnv};
use str_to_bool::str_to_bool;

// Conf, and perhaps Error, is really the only thing users should use, but the derive macro needs these other types.
pub use error::Error;
pub use traits::Conf;

#[doc(hidden)]
pub use conf_context::{ConfContext, ValueSource};
#[doc(hidden)]
pub use error::InnerError;
#[doc(hidden)]
pub use parser::{ParsedArgs, ParserConfig};
#[doc(hidden)]
pub use program_option::{ParseType, ProgramOption};

// This is used by tests
#[doc(hidden)]
pub use parser::Parser;

#[doc(hidden)]
pub use conf_derive::{self, *};

// CowStr is used internally mainly because using it allows us to construct ProgramOption in a const way from string literals,
// but also to modify them if they have to be flattened into something. It also means we can declare ProgramOptions like Help and Version
// as static const variables, so we can handle them the same as everything else and avoid some borrow-checker issues.
type CowStr = std::borrow::Cow<'static, str>;
#[doc(hidden)]
pub const fn const_str(src: &'static str) -> CowStr {
    CowStr::Borrowed(src)
}
