use crate::{parse_env, Conf, ConfContext, Error, InnerError, ParsedArgs, ParsedEnv};
use std::{ffi::OsString, marker::PhantomData};

/// A builder which collects config value sources for the parse.
///
/// Use any of [`ConfBuilder::args`], [`ConfBuilder::env`], [`ConfBuilder::doc`] to set sources,
/// and then call one of [`ConfBuilder::parse`] or [`ConfBuilder::try_parse`].
///
/// If `args` is not called, the default source is `std::env::args_os`.
/// If `env` is not called, the default source is `std::env::vars_os`.
pub struct ConfBuilder<S>
where
    S: Conf,
{
    collected_env: ParsedEnv,
    inited_env: bool,
    collected_args: Vec<OsString>,
    inited_args: bool,
    _marker: PhantomData<fn() -> S>,
}

impl<S> Default for ConfBuilder<S>
where
    S: Conf,
{
    fn default() -> Self {
        Self {
            collected_env: Default::default(),
            inited_env: false,
            collected_args: Default::default(),
            inited_args: false,
            _marker: Default::default(),
        }
    }
}

impl<S> ConfBuilder<S>
where
    S: Conf,
{
    /// Set the CLI args used in this parse
    pub fn args(mut self, args: impl IntoIterator<Item: Into<OsString>>) -> Self {
        assert!(!self.inited_args, "Cannot set args twice");
        self.collected_args = args.into_iter().map(Into::into).collect();
        self.inited_args = true;
        self
    }

    /// Set the env vars used in this parse
    pub fn env<K, V>(mut self, env: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        assert!(!self.inited_env, "Cannot set env twice");
        self.collected_env = parse_env(env);
        self.inited_env = true;
        self
    }

    /// Parse based on supplied sources (or falling back to defaults), and exiting the program
    /// with errors logged to stderr if parsing fails.
    pub fn parse(self) -> S {
        match self.try_parse() {
            Ok(result) => result,
            Err(err) => err.exit(),
        }
    }

    /// Try to parse an instance based on supplied sources (or falling back to defaults),
    /// returning an error if parsing fails.
    pub fn try_parse(self) -> Result<S, Error> {
        let (parsed_env, args) = self.into_tuple();

        let parser = S::get_parser(&parsed_env)?;
        let arg_matches = parser.parse(args)?;
        let parsed_args = ParsedArgs::new(&arg_matches, &parser);
        let conf_context = ConfContext::new(parsed_args, &parsed_env);
        S::from_conf_context(conf_context)
            .map_err(|errs| InnerError::vec_to_clap_error(errs, parser.get_command()))
    }

    /// Convert self into an args, env tuple, after setting defaults from std::env::* and such
    /// if anything was not inited
    pub(crate) fn into_tuple(mut self) -> (ParsedEnv, Vec<OsString>) {
        if !self.inited_args {
            self = self.args(std::env::args_os());
        }
        if !self.inited_env {
            self = self.env(std::env::vars_os());
        }

        (self.collected_env, self.collected_args)
    }
}
