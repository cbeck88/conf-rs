//! Facilities for introspecting on Conf structures
use core::fmt::Display;

/// Public interface to metadata about a program option
pub trait ProgramOptionMeta {
    /// The id of the program option. This is its generally its "rust path" in the config object.
    fn id(&self) -> &dyn Display;
    /// The description of the program option. This is generally its doc string, plus any prefixing.
    /// This is also displayed in the help text for the program option.
    fn description(&self) -> &dyn Display;
    /// The short-form switch (-f, -h) of the option, if any.
    fn short_form(&self) -> Option<char>;
    /// The long-form switch (--file, --help) of the option, if any.
    fn long_form(&self) -> Option<&dyn Display>;
    /// True the option can be specified as a positional command line argument.
    fn is_positional(&self) -> bool;
    /// The environment variable associated to this option, if any.
    fn env_form(&self) -> Option<&dyn Display>;
    /// True if this option could be read from the serde document.
    fn has_serde_source(&self) -> bool;
    /// True if this option is required to appear in one of the input sources (i.e. it's an error if doesn't)
    fn is_required(&self) -> bool;
}

/// Describes the source of the value when a program option is assigned a value
#[non_exhaustive]
pub enum ValueSource<'a> {
    /// The value came from command-line arguments
    #[non_exhaustive]
    Args {},
    /// The value came from an environment variable
    #[non_exhaustive]
    Env {
        /// The env var that was read
        var: &'a dyn Display,
    },
    /// The value came from a document
    #[non_exhaustive]
    Document {
        /// The name of the document that was read
        name: &'a dyn Display,
    },
    /// The value came from a default setting
    #[non_exhaustive]
    Default {},
}

/// A record of how a program option was assigned a value
pub trait ConfigEvent {
    /// Get the program option
    fn program_option(&self) -> &dyn ProgramOptionMeta;
    /// Get the value source
    fn value_source(&self) -> ValueSource<'_>;
}

/// A callback that receives ConfigEvents
#[doc(hidden)]
pub type ConfigLogger<'a> = &'a mut dyn FnMut(&dyn ConfigEvent);
