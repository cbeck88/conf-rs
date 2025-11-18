use crate::{ConfSerde, ConfSerdeContext, IdentString, InnerError};
use core::fmt;
use serde::de::{Deserialize, DeserializeSeed, Deserializer};

/// A handle to a subset of the [`serde::de::MapAccess`] functionality.
/// This handle allows one to call `next_value` or `next_value_seed` *once*, with
/// whatever type is desired.
///
/// This is useful when the type should depend on the key,
/// and how exactly is decided by code that is generated elsewhere.
/// The `NextValueProducer` can be passed to that code without risk of corrupting
/// the `MapAccess`, since the caller has a static guarantee about how and how
/// many times the receiver can use the `MapAccess`.
#[doc(hidden)]
pub trait NextValueProducer<'de>: Sized {
    type Error: serde::de::Error;

    fn next_value_seed<S>(self, seed: S) -> Result<S::Value, Self::Error>
    where
        S: DeserializeSeed<'de>;

    fn next_value<V>(self) -> Result<V, Self::Error>
    where
        V: Deserialize<'de>,
    {
        self.next_value_seed(core::marker::PhantomData)
    }
}

impl<'de, MA> NextValueProducer<'de> for &mut MA
where
    MA: serde::de::MapAccess<'de>,
{
    type Error = MA::Error;

    fn next_value_seed<S>(self, seed: S) -> Result<S::Value, Self::Error>
    where
        S: DeserializeSeed<'de>,
    {
        MA::next_value_seed(self, seed)
    }
}

/// In order to implement serde(flatten) properly, we need to allow for the idea that:
///
/// * struct A contains two flattened children struct B and struct C
/// * some fields are produced by serde::de::MapAccess<'de> which go to A, then which go to B,
///   then which go to C, then which go back to B again, then which go back to C, etc.
///
/// This means that the code we generate for B cannot take a deserializer or similar and generate
/// an entire B in one shot or produce an error. It needs to be able to start the process for B,
/// then go somewhere, else, and then return and continue with B. This means that while initing B,
/// we need to have a state machine.
///
/// The state machine is advanced by passing it (&str, NextValueProducer) which it wants
/// It can be finalized when there are no more key-value pairs, producing Value or a set of errors.
#[doc(hidden)]
pub trait InitializationStateMachine<'de>: Sized {
    type Value;

    fn wants_key(&self, key: &str) -> bool;
    fn next<NVP>(self, key: &str, next_value_producer: NVP) -> Self
    where
        NVP: NextValueProducer<'de>;
    fn finalize(self) -> Result<Self::Value, Vec<InnerError>>;
}

/// Wrapper type to treat an initialization state machine as a serde::de::Visitor impl.
/// This is only here to avoid using blanket implementations, which can become problematic.
///
/// An ISM can be used in phases, but it can also obviously be used in one shot from a deserializer.
/// That's used when implementing DeserializeSeed on a struct.
#[doc(hidden)]
pub struct AsVisitor<M> {
    pub machine: M,
    pub expecting_fn: fn(&mut fmt::Formatter) -> fmt::Result,
}

impl<'de, M> serde::de::Visitor<'de> for AsVisitor<M>
where
    M: InitializationStateMachine<'de>,
{
    type Value = Result<M::Value, Vec<InnerError>>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self.expecting_fn)(f)
    }

    fn visit_map<MA>(self, mut map_access: MA) -> Result<Self::Value, MA::Error>
    where
        MA: serde::de::MapAccess<'de>,
    {
        let Self { mut machine, .. } = self;

        while let Some(key) = map_access.next_key::<IdentString>()? {
            machine = machine.next(key.as_str(), &mut map_access);
        }
        Ok(machine.finalize())
    }
}

/// An object which can serve as a seed to generate any struct implementing ConfSerde.
///
/// It's a bit simpler to code-gen the initialization state machine than to code-gen this object,
/// and the initialization state machine needs to be reified in order to implement serde(flatten).
pub struct ConfSerdeSeed<'a, V> {
    ctxt: ConfSerdeContext<'a>,
    _marker: core::marker::PhantomData<fn() -> V>,
}

impl<'a, V> From<ConfSerdeContext<'a>> for ConfSerdeSeed<'a, V> {
    fn from(ctxt: ConfSerdeContext<'a>) -> Self {
        Self {
            ctxt,
            _marker: Default::default(),
        }
    }
}

impl<'de, 'a, V> DeserializeSeed<'de> for ConfSerdeSeed<'a, V>
where
    V: ConfSerde,
{
    type Value = Result<V, Vec<InnerError>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let struct_name = V::STRUCT_NAME;
        let expecting_fn = V::expecting;

        let doc_name = self.ctxt.document_name;
        let machine = V::ISM::<'a>::from(self.ctxt);
        let visitor = AsVisitor {
            machine,
            expecting_fn,
        };

        let deserialize_result = if let Some(keys) = V::STRUCT_KEYS {
            deserializer.deserialize_struct(struct_name, keys, visitor)
        } else {
            deserializer.deserialize_map(visitor)
        };

        Ok(match deserialize_result {
            Ok(result) => result,
            Err(err) => Err(vec![InnerError::serde(doc_name, struct_name, err)]),
        })
    }
}

/// A wrapper around an [`InitializationStateMachine`] that strips a prefix from keys.
///
/// This is used to implement `serde(flatten(prefix))`, where keys in the serde document
/// have a common prefix (e.g., `database_host`, `database_port`) that should be stripped
/// before being passed to the inner state machine.
///
/// # Example
///
/// If you have a `DatabaseConfig` struct with fields `host` and `port`, and you use
/// `#[conf(flatten, serde(flatten(prefix)))]`, then the JSON keys would be:
/// - `database_host` (stripped to `host`)
/// - `database_port` (stripped to `port`)
///
/// Where `database_` is the snake_case of the field name plus an underscore.
#[doc(hidden)]
pub struct PrefixStrippingStateMachine<'a, M> {
    prefix: &'a str,
    inner: M,
}

impl<'a, M> PrefixStrippingStateMachine<'a, M> {
    /// Create a new prefix-stripping wrapper around an inner state machine.
    ///
    /// The `prefix` should include the trailing underscore (e.g., `"database_"`).
    pub fn new(prefix: &'a str, inner: M) -> Self {
        Self { prefix, inner }
    }
}

impl<'de, 'a, M> InitializationStateMachine<'de> for PrefixStrippingStateMachine<'a, M>
where
    M: InitializationStateMachine<'de>,
{
    type Value = M::Value;

    fn wants_key(&self, key: &str) -> bool {
        if let Some(stripped_key) = key.strip_prefix(self.prefix) {
            self.inner.wants_key(stripped_key)
        } else {
            false
        }
    }

    fn next<NVP>(self, key: &str, next_value_producer: NVP) -> Self
    where
        NVP: NextValueProducer<'de>,
    {
        // Strip the prefix and pass to inner machine
        let stripped_key = key.strip_prefix(self.prefix).unwrap_or_else(|| {
            panic!("key '{}' does not start with prefix '{}'", key, self.prefix)
        });

        Self {
            prefix: self.prefix,
            inner: self.inner.next(stripped_key, next_value_producer),
        }
    }

    fn finalize(self) -> Result<Self::Value, Vec<InnerError>> {
        self.inner.finalize()
    }
}
