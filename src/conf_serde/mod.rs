//! Use a serde Deserializer as a value-soure for layered config.
//!
//! We discussed extensively in MOTIVATION.md why serde *deserializer* libraries
//! are unable to cause serde to return multiple errors when deserialization fails.
//! We also discussed why the ability to do that is so important to us.
//!
//! In this module, we create a way that we *can* return multiple errors when walking the
//! `Deserializer`, as long as we control the `Deserialize` side of things.
//! Since we control the proc-macro here, we have such control.
//!
//! We can even incorporate arbitrary other data, like a `ConfContext`, into the walk,
//! and use other value sources besides the `Deserializer` when initializing the struct.
//!
//! The main idea is to use the `DeserializeSeed` trait from `serde`, implement it
//! on (a variation of) ConfContext, and set `type Value = Result<S, Vec<InnerError>>;`
//! This means that when we implement `DeseralizeSeed::deserialize`, we get all the `ConfContext`,
//! meaning parsed args, env, any thing else that we want, and we get the `Deserializer`.
//! The type we have to return is `Result<Result<S, Vec<InnerError>>, D::Error>`, but if
//! we simply always return the `Ok` variant, then we basically have exactly the signature that
//! we wanted to implement.
//!
//! So we can inspect all this data, walk the deserializer and see what it has, then walk the
//! fields it didn't have any data for, and initialize everything according to the hierachical
//! config priority. We can collect as many errors as we need to, and when we return, we always
//! return `Ok(Ok(instance))` or `Ok(Err(errors))`. We can also use this same `DeserializeSeed`
//! trick when we recurse into a substructure, because `serde::de::MapAccess` allows us to invoke
//! the `DeserializeSeed` API however we want when recursing.
//!
//! At the end, the `ConfBuilder` is able to manage the error trickery we performed here and
//! return what the user was expecting, so none of this has any negative impact on the public API.
//!
//! And the plus side is, we have only loose coupling with all the file-reading and format-parsing
//! code. Users can bring their own stuff, whatever `serde_json` or `serde_json_lenient` or any
//! other variation, and configure it however they want. And conf has no explicit dependencies on
//! anything that reads a file.
//!
//! ***
//!
//! Note that due to this technique, you will get better error reporting from
//! using `Conf` to deserialize a large structure from a `serde::Deserializer` than
//! `serde::Deserialize` would be able to (!), because we can collect errors from all over the
//! structure.
//!
//! However, the technique has some limitations -- it has to be acceptable to type-erase
//! the serde errors. In this crate, we are mapping all the errors that can occur to
//! `conf::InnerError` and then aggregating them as one `clap::Error` essentially. If you wanted to
//! generalize the technique, and you don't want to break compat with existing serde Deserializers,
//! you would need to use `Box<dyn Error>` or something, but `serde` doesn't give you `'static` on
//! the error types, or convertibility between e.g. Deserializer errors and MapAccess errors.
//!
//! Another thing is, this is only really a good technique when you are deserializing a
//! self-describing format, like json, yaml, toml, etc, and you deserialize their type from
//! bytes-on-the-wire first. If you try to pass e.g. `serde_json::Deserializer::from_reader` as the
//! `Deserializer` here, you may get worse error reporting if the file is not well-formed
//! potentially, because once you have junk bytes on the wire and the wire protocol isn't working
//! right, trying to read more things is probably futile and just creates noise.
//!
//! Another drawback of this approach is, because we aren't using the `serde::Deserialize` derive
//! macro from `serde_derive`, we don't automatically get support for any of the `#[serde(...)]`
//! annotations. However, this is also a plus in some sense, because for things like
//! `#[serde(flatten)]`, we don't inherit any of the limitations or bugs, and we can choose to
//! implement it in a way that makes sense for our use-case later.
mod builder;
pub use builder::ConfSerdeBuilder;

mod traits;
pub use traits::{ConfSerde, ConfSerdeContext, NextValueProducer, SubcommandsSerde};

/// Helper for deserializing serde MapAccess keys in a struct visitor.
/// Similar to `String`, but uses "Deserializer::deserializer_identifier" to hint.
#[doc(hidden)]
pub struct IdentString {
    inner: String,
}

impl IdentString {
    /// Get identifier as a str
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl<'de> serde::de::Deserialize<'de> for IdentString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use core::fmt;
        use serde::de;

        struct Visitor {}
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = IdentString;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a field identifier")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(IdentString {
                    inner: s.to_owned(),
                })
            }
        }

        deserializer.deserialize_identifier(Visitor {})
    }
}
