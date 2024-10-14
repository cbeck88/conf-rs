use crate::{
    Conf, ConfBuilder, ConfContext, ConfSerde, ConfSerdeContext, Error, InnerError, ParsedArgs,
};
use serde::de::{DeserializeSeed, Deserializer};
use std::{ffi::OsString, marker::PhantomData};

impl<S> ConfBuilder<S>
where
    S: ConfSerde,
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
    ) -> ConfSerdeBuilder<'de, S, D> {
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
pub struct ConfSerdeBuilder<'de, S, D>
where
    S: ConfSerde,
    D: Deserializer<'de>,
{
    inner: ConfBuilder<S>,
    document_name: String,
    document: D,
    _marker: PhantomData<&'de u8>,
}

impl<'de, S, D> ConfSerdeBuilder<'de, S, D>
where
    S: ConfSerde,
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
            _marker,
        } = self;
        let (parsed_env, args) = inner.into_tuple();

        let parser = <S as Conf>::get_parser(&parsed_env)?;
        let arg_matches = parser.parse(args)?;
        let parsed_args = ParsedArgs::new(&arg_matches, &parser);
        let conf_context = ConfContext::new(parsed_args, &parsed_env);
        let conf_serde_context = ConfSerdeContext::new(conf_context, document_name.as_str());
        let seed = <S as ConfSerde>::Seed::from(conf_serde_context);
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
