use crate::{
    Conf, ConfBuilder, ConfContext, ConfSerde, ConfSerdeContext, ConfSerdeSeed, Error, InnerError,
    ParsedArgs, introspection::ConfigEvent,
};
use serde::de::{DeserializeSeed, Deserializer};
use std::{cell::RefCell, ffi::OsString, marker::PhantomData};

impl<S, F> ConfBuilder<S, F>
where
    S: ConfSerde,
    F: FnMut(&dyn ConfigEvent),
{
    /// Set the document used in this parse.
    ///
    /// Requires a name for the document and a serde Deserializer representing the content.
    ///
    /// The name is typically the name of a file, and is used in error messages.
    ///
    /// The deserializer is, for example, a serde_json::Value, serde_yaml::Value, figment::Value,
    /// etc. which you have already loaded from disk and parsed in an unstructured way.
    pub fn doc<'de, D: Deserializer<'de>>(
        self,
        document_name: impl Into<String>,
        deserializer: D,
    ) -> ConfSerdeBuilder<'de, D, S, F> {
        ConfSerdeBuilder {
            inner: self,
            document_name: document_name.into(),
            document: deserializer,
            _marker: Default::default(),
        }
    }
}

/// A ConfBuilder which additionally has serde-document content installed.
///
/// This is only allowed when the target struct supports serde, i.e. has `#[conf(serde)]` attribute.
pub struct ConfSerdeBuilder<'de, D, S, F>
where
    S: ConfSerde,
    F: FnMut(&dyn ConfigEvent),
    D: Deserializer<'de>,
{
    inner: ConfBuilder<S, F>,
    document_name: String,
    document: D,
    _marker: PhantomData<&'de u8>,
}

impl<'de, D, S, F> ConfSerdeBuilder<'de, D, S, F>
where
    S: ConfSerde,
    F: FnMut(&dyn ConfigEvent),
    D: Deserializer<'de>,
{
    /// Set the env vars used in this parse
    pub fn env<K, V>(mut self, env: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.inner = self.inner.env(env);
        self
    }

    /// Set the CLI args used in this parse
    pub fn args(mut self, args: impl IntoIterator<Item: Into<OsString>>) -> Self {
        self.inner = self.inner.args(args);
        self
    }

    /// Set the config logger used in this parse
    /// This enables introspection on the config process, so that it can be determined
    /// which program options are configured via which value sources.
    pub fn config_logger<F2: FnMut(&dyn ConfigEvent)>(
        self,
        f: F2,
    ) -> ConfSerdeBuilder<'de, D, S, F2> {
        ConfSerdeBuilder {
            inner: self.inner.config_logger(f),
            document_name: self.document_name,
            document: self.document,
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
        let Self {
            inner,
            document,
            document_name,
            ..
        } = self;
        let (parsed_env, args, mut config_logger) = inner.into_tuple();
        let config_logger_refcell = config_logger
            .as_mut()
            .map(|cb| RefCell::new(&mut *cb as &mut dyn FnMut(&dyn ConfigEvent)));
        let config_logger = config_logger_refcell.as_ref();

        let mut parser = <S as Conf>::get_parser(&parsed_env)?;
        let arg_matches = parser.parse(args)?;
        let parsed_args = ParsedArgs::new(&arg_matches, &parser);
        let conf_context = ConfContext::new(parsed_args, &parsed_env, config_logger);
        let conf_serde_context = ConfSerdeContext::new(conf_context, document_name.as_str());
        let seed = ConfSerdeSeed::<S>::from(conf_serde_context);
        // Code gen should produce:
        // impl<'de> DeserializeSeed for Seed {
        //   type Value = Result<Self, Vec<conf::InnerError>>;
        //   ...
        // }
        // So that the result of deserialize call is Result<Result<Self, Vec<InnerError>>, D::Error>
        DeserializeSeed::<'de>::deserialize(seed, document)
            .expect("Internal error, Deserializer Error should not be returned here")
            .map_err(|errs| InnerError::vec_to_clap_error(errs, parser.get_command()))
    }
}
