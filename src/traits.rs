use crate::{parse_env, ConfContext, Error, Parser, ParserConfig, ProgramOption};
use std::ffi::OsString;

/// The Conf trait is implemented by types that represent a collection of config parsed on startup, and is modeled
/// on `clap::Parser`. Users usually call `parse` or another of these functions on their
/// config structure in `main()`. The `parse_from` version is mainly intended to be used
/// in tests.
pub trait Conf: Sized {
    /// Parse self from the process CLI args and environment, and exit the program with a help message if we cannot.
    #[inline]
    fn parse() -> Self {
        match Self::try_parse() {
            Ok(result) => result,
            Err(err) => err.exit(),
        }
    }

    /// Try to parse self from the process CLI args and environment, and return an error if we cannot.
    #[inline]
    fn try_parse() -> Result<Self, Error> {
        Self::try_parse_from(std::env::args_os(), std::env::vars_os())
    }

    /// Parse self from given containers which stand in for the process args and environment, and exit the program with a help message if we cannot.
    #[inline]
    fn parse_from<T, K, V>(
        args_os: impl IntoIterator<Item = T>,
        env_vars_os: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        T: Into<OsString> + Clone,
        K: Into<OsString> + Clone,
        V: Into<OsString> + Clone,
    {
        match Self::try_parse_from(args_os, env_vars_os) {
            Ok(result) => result,
            Err(err) => err.exit(),
        }
    }

    /// Try to parse self from given containers which stand in for the process args and environment, and return an error if we cannot.
    fn try_parse_from<T, K, V>(
        args_os: impl IntoIterator<Item = T>,
        env_vars_os: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Self, Error>
    where
        T: Into<OsString> + Clone,
        K: Into<OsString> + Clone,
        V: Into<OsString> + Clone,
    {
        let (parser_config, program_options) = Self::get_program_options()?;
        let parsed_env = parse_env(env_vars_os);
        let parser = Parser::new(parser_config, &program_options, &parsed_env)?;
        let parsed_args = parser.parse(args_os)?;
        let conf_context = ConfContext::new(&parsed_args, &parsed_env)?;
        Self::from_conf_context(conf_context)
    }

    // Get the program options this object declares, and associated help info
    // This is generally implemented using the derive macros.
    // Users shouldn't generally call this, because the returned data is implementation details,
    // and may change without a semver breaking change to the crate version.
    #[doc(hidden)]
    fn get_program_options() -> Result<(ParserConfig, Vec<ProgramOption>), Error>;
    // Try to parse an instance of self from a given parser context
    // This is generally implemented using the derive macros.
    // Users generally can't call this, because ConfContext is not constructible by any public APIs.
    #[doc(hidden)]
    fn from_conf_context(conf_context: ConfContext<'_>) -> Result<Self, Error>;
}
