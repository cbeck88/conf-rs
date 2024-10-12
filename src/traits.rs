use crate::{
    parse_env, ConfContext, ConfValueSource, Error, InnerError, ParsedArgs, ParsedEnv, Parser,
    ParserConfig, ProgramOption,
};
use std::ffi::OsString;

/// The Conf trait is implemented by types that represent a collection of config parsed on startup,
/// and is modeled on `clap::Parser`. Users usually call `parse` or another of these functions on
/// their config structure in `main()`.
///
/// Hand-written implementations of this trait are not supported.
#[doc = include_str!("../REFERENCE_derive_conf.md")]
pub trait Conf: Sized {
    /// Parse self from the process CLI args and environment, and exit the program with a help
    /// message if we cannot.
    #[inline]
    fn parse() -> Self {
        match Self::try_parse() {
            Ok(result) => result,
            Err(err) => err.exit(),
        }
    }

    /// Try to parse self from the process CLI args and environment, and return an error if we
    /// cannot.
    #[inline]
    fn try_parse() -> Result<Self, Error> {
        Self::try_parse_from(std::env::args_os(), std::env::vars_os())
    }

    /// Parse self from given containers which stand in for the process args and environment, and
    /// exit the program with a help message if we cannot. This function's behavior is isolated
    /// from the values of `std::env::args_os` and `std::env::vars_os`.
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

    /// Try to parse self from given containers which stand in for the process args and environment,
    /// and return an error if we cannot.
    fn try_parse_from<T, K, V>(
        args_os: impl IntoIterator<Item = T>,
        env_vars_os: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Self, Error>
    where
        T: Into<OsString> + Clone,
        K: Into<OsString> + Clone,
        V: Into<OsString> + Clone,
    {
        let parsed_env = parse_env(env_vars_os);
        let parser = Self::get_parser(&parsed_env)?;
        let arg_matches = parser.parse(args_os)?;
        let parsed_args = ParsedArgs::new(&arg_matches, &parser);
        let conf_context = ConfContext::new(parsed_args, &parsed_env);
        Self::from_conf_context(conf_context)
            .map_err(|errs| InnerError::vec_to_clap_error(errs, parser.get_command()))
    }

    // Construct a conf::Parser object appropriate for this Conf.
    // This requires the parsed_env because that is used in help text.
    // This Parser may be used in Conf::try_parse_from, or may be used to implement
    // Subcommands::get_commands.
    #[doc(hidden)]
    fn get_parser(parsed_env: &ParsedEnv) -> Result<Parser<'_>, Error> {
        let parser_config = Self::get_parser_config()?;
        let program_options = Self::get_program_options()?;
        let subcommands = Self::get_subcommands(parsed_env)?;
        Parser::new(parser_config, program_options, subcommands, parsed_env)
    }

    // Get the parser config associated to this Conf.
    // This basically means, top-level options that affect parsing or clap setup, but not any of the
    // program options specifically.
    // This is implemented using the derive macros.
    // Users shouldn't generally call this, because the returned data is implementation details,
    // and may change without a semver breaking change to the crate version.
    #[doc(hidden)]
    fn get_parser_config() -> Result<ParserConfig, Error>;
    // Get the program options this Conf declares, and associated help info etc, including flattened
    // fields. This is implemented using the derive macros.
    // Users shouldn't generally call this, because the returned data is implementation details,
    // and may change without a semver breaking change to the crate version.
    #[doc(hidden)]
    fn get_program_options() -> Result<&'static [ProgramOption], Error>;
    // Get the subcommands that are declared on this Conf.
    //
    // These come from `conf(subcommand)` being used on a field, and `derive(Subcommand)` being used
    // on the enum type of that field.
    //
    // This requires ParsedEnv because a command contains a help page, and the env influences that.
    #[doc(hidden)]
    fn get_subcommands(parsed_env: &ParsedEnv) -> Result<Vec<Parser<'_>>, Error>;
    // Try to parse an instance of self from a given parser context
    // This is implemented using the derive macros.
    // Users generally can't call this, because ConfContext is not constructible by any public APIs.
    #[doc(hidden)]
    fn from_conf_context(conf_context: ConfContext<'_>) -> Result<Self, Vec<InnerError>>;
    // Check if any program options from this Conf appeared in given conf context, before attempting
    // to parse it. Here "appeared" means that it has a value, and the value was not a default
    // value. Returns an id (and value source) that can be used with conf_context to get the
    // program option that appeared Note that this id is a relative id relative to thsi object
    // and this conf context, not an absolute id.
    //
    // This is used to implement flatten-optional, and also to get error details when a one-of
    // constraint fails Users generally can't call this, because ConfContext is not
    // constructible by any public APIs.
    #[doc(hidden)]
    fn any_program_options_appeared<'a>(
        conf_context: &ConfContext<'a>,
    ) -> Result<Option<(&'a str, ConfValueSource<&'a str>)>, InnerError> {
        // This unwrap is unfortunate but this code is only called when an earlier call to
        // Self::get_program_options has succeeded, since we have to call that to
        // instantiate the parser, and we have to do that before getting a ConfContext.
        // The only place in the library where a `ConfContext` is created where one doesn't already
        // exist is in `try_parse_from`, and the ConfContext::new function is pub(crate).
        // And we have to call get_program_options before that point, which calls it
        // recursively on all the constituent structures.
        // So I don't think this unwrap will panic unless get_program_options is implemented in a
        // non-deterministic way, which it shouldn't be.
        let program_options = Self::get_program_options().unwrap();
        for opt in program_options {
            if let Some(value_source) = conf_context.option_appears(&opt.id)? {
                return Ok(Some((&opt.id, value_source)));
            }
        }
        Ok(None)
    }
    // Get the name used for this group of options in error messages.
    // Generally this is the struct identifier
    #[doc(hidden)]
    fn get_name() -> &'static str;
}

/// The Subcommands trait represents one or more subcommands, and is derived on Enums.
///
/// Each subcommand is an enum variant containing a Conf structure.
///
/// The subcommand name is the name of the enum variant.
///in
/// A Subcommands enum can then be added as a field to a top-level Conf structure and marked using
/// the `#[conf(subcommands)]` attribute.
///
/// Hand-written implementations of this trait are not supported.
#[doc = include_str!("../REFERENCE_derive_subcommands.md")]
pub trait Subcommands: Sized {
    // Get the subcommands associated to this enum.
    // This is generally done by calling get_parser for each variant, and then get_command on the
    // parser. The Command::name should then be set based on the name of the enum variant.
    #[doc(hidden)]
    fn get_parsers(env: &ParsedEnv) -> Result<Vec<Parser<'_>>, Error>;

    // Get the subcommand names associated to this enum, for error messages
    #[doc(hidden)]
    fn get_subcommand_names() -> &'static [&'static str];

    // Construct Self from a command name and a conf context for the corresponding subcommand
    #[doc(hidden)]
    fn from_conf_context(
        command_name: String,
        conf_context: ConfContext<'_>,
    ) -> Result<Self, Vec<InnerError>>;
}
