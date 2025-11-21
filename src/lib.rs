//! A `derive`-based config parser aimed at the practically-minded web developer building large web projects.
//!
//! To use `conf`, use the `#[derive(Conf)]` proc macro on your configuration struct.
//! Then call a [`Conf`] trait function to parse your configuration struct.
//! Proc macro attributes are documented there.
//!
//! For hierarchical config, add the `#[conf(serde)]` annotation to your configuration struct.
//! Then use [`Conf::conf_builder`] to get a builder object, and call `ConfBuilder::doc`
//! to supply a "document" representing the config file in any serde-compatible format.
//! A typical example might be a `serde_json::Value`.
//! Then call `ConfBuilder::parse` or similar on the builder.
//!
//! See the [readme] for an overview.
#![deny(unsafe_code)]
#![deny(missing_docs)]

pub mod readme;

mod builder;
mod conf_context;
mod error;
mod find_parameter;
pub mod introspection;
#[doc(hidden)]
pub mod lazybuf;
mod parse_env;
mod parser;
mod program_option;
mod str_to_bool;
mod styles;
mod traits;

// These are not needed by users or by generated code
use conf_context::FlattenedOptionalDebugInfo;
use parse_env::parse_env;
use parser::ParsedArgs;
use str_to_bool::str_to_bool;

// These exports represent public API.
pub use builder::ConfBuilder;
pub use error::Error;
pub use find_parameter::find_parameter;
pub use styles::Styles;
pub use traits::{Conf, Subcommands};

// Re-export anstyle for users to create Style objects
pub use anstyle;

// Export conf_derive proc-macros unconditionally. Their docs are on the traits that they
// produce implementations for.
#[doc(hidden)]
pub use conf_derive::{self, *};

// The derive macro needs these other types, so they are exported, but doc(hidden).
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

// The serde feature brings in some more types and traits
#[cfg(feature = "serde")]
mod conf_serde;
// The ConfSerde trait and builder are publicly documented
#[cfg(feature = "serde")]
pub use conf_serde::{ConfSerde, ConfSerdeBuilder};
// These are internals used by the derive macro.
#[doc(hidden)]
#[cfg(feature = "serde")]
pub use conf_serde::{
    ConfSerdeContext, ConfSerdeSeed, IdentString, InitializationStateMachine, NextValueProducer,
    OptionalStateMachine, PrefixStrippingStateMachine, SubcommandsSerde,
};
// Re-export serde crate for the proc macro
#[doc(hidden)]
#[cfg(feature = "serde")]
pub use serde::{self, *};

// CowStr is used internally mainly because using it allows us to construct ProgramOption in a const
// way from string literals, but also to modify them if they have to be flattened into something.
// In the future we could use a rope or something instead, but this is fine.
type CowStr = std::borrow::Cow<'static, str>;

// Helper for some of the proc-macro code-gen
// This lets you get the inner-type of a Vec<T> no matter how Vec<T> is spelled
// (Vec, std::vec::Vec, alloc::vec::Vec) or aliased by user code.
// This is normally difficult to do directly from a proc-macro, which can only see the tokens.
#[doc(hidden)]
pub trait InnerTypeHelper {
    type Ty;
}

impl<T> InnerTypeHelper for Vec<T> {
    type Ty = T;
}
