use crate::{
    Conf, ConfContext, Error, InnerError, ParsedArgs, ParsedEnv, introspection::ConfigEvent,
    parse_env,
};
use std::{cell::RefCell, ffi::OsString, marker::PhantomData};

/// A builder which collects config value sources for the parse.
///
/// Use any of [`ConfBuilder::args`], [`ConfBuilder::env`], [`ConfBuilder::doc`] to set sources,
/// if desired use `ConfBuilder::config_logger` to set a callback,
/// and then call one of [`ConfBuilder::parse`] or [`ConfBuilder::try_parse`].
///
/// If `args` is not called, the default source is `std::env::args_os`.
/// If `env` is not called, the default source is `std::env::vars_os`.
pub struct ConfBuilder<S, F = fn(&dyn ConfigEvent)>
where
    S: Conf,
    F: FnMut(&dyn ConfigEvent),
{
    collected_env: ParsedEnv,
    inited_env: bool,
    collected_args: Vec<OsString>,
    inited_args: bool,
    config_logger: Option<F>,
    _marker: PhantomData<fn() -> S>,
}

impl<S, F> Default for ConfBuilder<S, F>
where
    S: Conf,
    F: FnMut(&dyn ConfigEvent),
{
    fn default() -> Self {
        Self {
            collected_env: Default::default(),
            inited_env: false,
            collected_args: Default::default(),
            inited_args: false,
            config_logger: None,
            _marker: Default::default(),
        }
    }
}

impl<S, F> ConfBuilder<S, F>
where
    S: Conf,
    F: FnMut(&dyn ConfigEvent),
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

    /// Set the config logger used in this parse
    /// This enables introspection on the config process, so that it can be determined
    /// which program options are configured via which value sources.
    pub fn config_logger<F2: FnMut(&dyn ConfigEvent)>(self, f: F2) -> ConfBuilder<S, F2> {
        assert!(
            self.config_logger.is_none(),
            "Cannot set config_logger twice"
        );
        ConfBuilder {
            collected_env: self.collected_env,
            inited_env: self.inited_env,
            collected_args: self.collected_args,
            inited_args: self.inited_args,
            config_logger: Some(f),
            _marker: PhantomData,
        }
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
        let (parsed_env, args, mut config_logger) = self.into_tuple();
        let config_logger_refcell = config_logger
            .as_mut()
            .map(|cb| RefCell::new(&mut *cb as &mut dyn FnMut(&dyn ConfigEvent)));
        let config_logger = config_logger_refcell.as_ref();

        let program_options = S::get_program_options();
        let mut parser = S::get_parser(&parsed_env)?;
        let arg_matches = parser.parse(args)?;
        let parsed_args = ParsedArgs::new(&arg_matches, &parser);
        let conf_context =
            ConfContext::new(parsed_args, &parsed_env, program_options, config_logger);
        S::from_conf_context(conf_context)
            .map_err(|errs| InnerError::vec_to_clap_error(errs, parser.get_command()))
    }

    /// Convert self into an args, env tuple, after setting defaults from std::env::* and such
    /// if anything was not inited
    pub(crate) fn into_tuple(mut self) -> (ParsedEnv, Vec<OsString>, Option<F>) {
        if !self.inited_args {
            self = self.args(std::env::args_os());
        }
        if !self.inited_env {
            self = self.env(std::env::vars_os());
        }

        (self.collected_env, self.collected_args, self.config_logger)
    }
}
