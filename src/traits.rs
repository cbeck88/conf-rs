use crate::{
    ConfBuilder, ConfContext, ConfValueSource, Error, InnerError, ParsedEnv, Parser, ParserConfig,
    ProgramOption, introspection, lazybuf::LazyBuf,
};
use std::ffi::OsString;

/// The Conf trait is implemented by types that represent a collection of config parsed on startup,
/// and is modeled on `clap::Parser`.
///
/// To use it, put `#[derive(Conf)]` on your config structure, and then call `Conf::parse` or
/// another of these functions in `main()`.
///
/// **Hand-written implementations of this trait are not supported**.
///
/// You should think of this trait as a "non-exhaustive trait" with hidden required items.
/// `conf` is free to add, modify, or remove these implementation details without considering it
/// a semver breaking change, so the only stable way to get an impl is via the derive macro.
#[doc = include_str!("../REFERENCE_derive_conf.md")]
pub trait Conf: Sized {
    /// Parse self from the process CLI args and environment, and exit the program with a help
    /// message if we cannot.
    #[inline]
    fn parse() -> Self {
        Self::conf_builder().parse()
    }

    /// Try to parse self from the process CLI args and environment, and return an error if we
    /// cannot.
    #[inline]
    fn try_parse() -> Result<Self, Error> {
        Self::conf_builder().try_parse()
    }

    /// Parse self from given containers which stand in for the process args and environment, and
    /// exit the program with a help message if we cannot. This function's behavior is isolated
    /// from the values of [`std::env::args_os`] and [`std::env::vars_os`].
    #[inline]
    fn parse_from<T, K, V>(
        args_os: impl IntoIterator<Item: Into<OsString>>,
        env_vars_os: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        Self::conf_builder().args(args_os).env(env_vars_os).parse()
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
        Self::conf_builder()
            .args(args_os)
            .env(env_vars_os)
            .try_parse()
    }

    /// Obtain a ConfBuilder, and use the builder API to parse.
    /// The builder API is needed if you want to use advanced features.
    fn conf_builder() -> ConfBuilder<Self> {
        Default::default()
    }

    /// Iterate over the program options declared in this Conf structure.
    /// This can be used for tasks such as generating an .env template.
    ///
    /// ```rust
    /// use conf::{Conf, introspection::ProgramOptionMeta};
    /// use std::net::SocketAddr;
    /// use std::time::Duration;
    ///
    /// #[derive(Conf)]
    /// pub struct Config {
    ///   /// Socket addr to listen to for HTTP
    ///   #[conf(long, env, default_value = "0.0.0.0:6666")]
    ///   pub http_listen_addr: SocketAddr,
    ///   /// Upstream service URL to connect to
    ///   #[conf(long, env, default_value = "http://upstream.com")]
    ///   pub upstream_service_url: String,
    ///   /// HTTP timeout for upstream connections (seconds)
    ///   #[conf(long, env, default_value = "5")]
    ///   pub http_timeout: u32,
    /// }
    ///
    /// let env_template = Config::program_options().filter_map(|opt| {
    ///     if let Some(env_form) = opt.env_form() {
    ///         let text = if let Some(desc) = opt.description() {
    ///             format!("# {desc}\n{env_form}=")
    ///         } else {
    ///             format!("{env_form}=")
    ///         };
    ///         Some(text)
    ///     } else {
    ///         None
    ///     }
    /// }).collect::<Vec<String>>().join("\n");
    ///
    /// let expected_output = r##"
    /// \# Socket addr to listen to for HTTP
    /// HTTP_LISTEN_ADDR=
    /// \# Upstream service URL to connect to
    /// UPSTREAM_SERVICE_URL=
    /// \# HTTP timeout for upstream connections (seconds)
    /// HTTP_TIMEOUT="##
    ///    .trim().replace(r"\#", "#"); // <- work around rustdoc handling of #
    ///
    /// assert_eq!(env_template, expected_output);
    fn program_options() -> impl Iterator<Item: introspection::ProgramOptionMeta> {
        Self::PROGRAM_OPTIONS.iter()
    }

    /// Run clap's debug assertions on the parser configuration for this struct.
    /// This is primarily used by the `#[conf(test)]` attribute to generate tests.
    #[doc(hidden)]
    fn parser_debug_asserts() {
        let parsed_env = ParsedEnv::default();
        let program_options = Self::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();
        let parser =
            Self::get_parser(&parsed_env, program_options).expect("Failed to create parser");
        parser.into_command().debug_assert();
    }

    /// Run debug assertions on this struct's fields recursively.
    /// This validates default_value parsing and recurses into flattened fields and subcommands.
    #[doc(hidden)]
    fn debug_asserts();

    // Construct a conf::Parser object appropriate for this Conf.
    // This requires the parsed_env because that is used in help text.
    // This Parser may be used in Conf::try_parse_from, or may be used to implement
    // Subcommands::get_commands.
    #[doc(hidden)]
    fn get_parser(
        parsed_env: &ParsedEnv,
        program_options: Vec<ProgramOption>,
    ) -> Result<Parser, Error> {
        let parser_config = Self::get_parser_config()?;
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

    // Program options this Conf declares (and recursive sources), without allocating.
    // Constructed by derive macros.
    // Users shouldn't generally call this, because the returned data is implementation details,
    // and may change without a semver breaking change to the crate version.
    #[doc(hidden)]
    const PROGRAM_OPTIONS: LazyBuf<ProgramOption>;
    // Get the subcommands that are declared on this Conf.
    //
    // These come from `conf(subcommand)` being used on a field, and `derive(Subcommand)` being used
    // on the enum type of that field.
    //
    // This requires ParsedEnv because a command contains a help page, and the env influences that.
    #[doc(hidden)]
    fn get_subcommands(parsed_env: &ParsedEnv) -> Result<Vec<Parser>, Error>;
    // Try to parse an instance of self from a given parser context
    // This is implemented using the derive macros.
    // Users generally can't call this, because ConfContext is not constructible by any public APIs.
    #[doc(hidden)]
    fn from_conf_context(conf_context: ConfContext<'_>) -> Result<Self, Vec<InnerError>>;
    // Check if any program options from this Conf appeared in given conf context, before attempting
    // to parse it. Here "appeared" means that it has a value, and the value was not a default
    // value. Returns an id (and value source) that can be used to look up the program option
    // that appeared. The returned id is an absolute id (fully prefixed).
    //
    // This is used to implement flatten-optional, and also to get error details when a one-of
    // constraint fails Users generally can't call this, because ConfContext is not
    // constructible by any public APIs.
    #[doc(hidden)]
    fn any_program_options_appeared<'a>(
        conf_context: &ConfContext<'a>,
    ) -> Result<Option<(&'a str, ConfValueSource<&'a str>)>, InnerError> {
        for (relative_id, opt) in conf_context.get_relevant_program_options() {
            if let Some(value_source) = conf_context.option_appears(relative_id)? {
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

/// The Subcommands trait represents one or more subcommands that can be added to a `Conf`
/// structure. To use it, put `#[derive(Subcommands)]` on your enum, and then add a
/// `#[conf(subcommands)]` field to your `Conf` structure whose type is your enum type, or
/// `Option<T>` where `T` is your enum type.
///
/// Each variant of the enum corresponds to a subcommand, and must contain a single un-named `Conf`
/// structure.
///
/// The subcommand name is the name of the enum variant, but you can use attributes to change this
/// name.
///
/// A Subcommands enum can then be added as a field to a top-level Conf structure and marked using
/// the `#[conf(subcommands)]` attribute.
///
/// **Hand-written implementations of this trait are not supported**.
///
/// You should think of this trait as a "non-exhaustive trait" with hidden required items.
/// `conf` is free to add, modify, or remove these implementation details without considering it
/// a semver breaking change, so the only stable way to get an impl is via the derive macro.
#[doc = include_str!("../REFERENCE_derive_subcommands.md")]
pub trait Subcommands: Sized {
    // Get the subcommands associated to this enum.
    // This is generally done by calling get_parser for each variant, and then get_command on the
    // parser. The Command::name should then be set based on the name of the enum variant.
    #[doc(hidden)]
    fn get_parsers(env: &ParsedEnv) -> Result<Vec<Parser>, Error>;

    // Get the subcommand names associated to this enum, for error messages
    #[doc(hidden)]
    fn get_subcommand_names() -> &'static [&'static str];

    // Construct Self from a command name and a conf context for the corresponding subcommand
    #[doc(hidden)]
    fn from_conf_context(
        command_name: String,
        conf_context: ConfContext<'_>,
    ) -> Result<Self, Vec<InnerError>>;

    /// Run debug assertions on all subcommand variants recursively.
    #[doc(hidden)]
    fn debug_asserts();
}
