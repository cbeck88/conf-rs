use crate::{ConfSerdeContext, IdentString, InnerError};
use core::fmt;
use serde::de::{Deserialize, DeserializeSeed};

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
/// The state machine is advanced by passing it (&str, NextValueProducer).
/// It can also declare which keys (&str) it's actually interested in.
/// It can be finalized when there are no more key-value pairs, producing Value or a set of errors.
///
/// TODO: Ideally, instead of wants_key(key: &str) -> bool, the state machine would have to declare
/// up front all of the keys it is interested in, in some format so that we can detect collisions.
/// However, the design of that is complex and it seems better to start with the simplest version
/// and iterate towards success.
#[doc(hidden)]
pub trait InitializationStateMachine<'de>: Sized {
    type Value;

    fn keys(&self) -> &'static [&'static str];
    fn next(&mut self, key: &str, next_value_producer: impl NextValueProducer<'de>);
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
    expecting_fn: fn(&mut fmt::Formatter) -> fmt::Result,
}

impl<'de, M> serde::de::Visitor<'de> for AsVisitor<M>
where
    M: InitializationStateMachine<'de>,
{
    type Value = M;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self.expecting_fn)(f)
    }

    fn visit_map<MA>(mut self, mut map_access: MA) -> Result<M, MA::Error>
    where
        MA: serde::de::MapAccess<'de>,
    {
        while let Some(key) = map_access.next_key::<IdentString>()? {
            self.machine.next(key.as_str(), &mut map_access);
        }
        Ok(self.machine)
    }
}

/// Wrapper type to treat an initialization state machine as a DeserializeSeed impl.
/// This is only here to avoid using blanket implementations, which can become problematic.
///
/// Note: AsSeed<M> implements DeserializeSeed, and calls to &AsSeed<M> which implements Visitor,
/// just for convenience.
///
/// The visitor returns M right before finalization should occur, then the finalization is
/// done in fn deserialize.
/// Moving the call to finalize allows for slightly cleaner error handling in the visitor function.
#[doc(hidden)]
pub struct AsSeed<'a, M> {
    pub struct_name: &'static str,
    pub expecting_fn: fn(&mut fmt::Formatter) -> fmt::Result,
    pub ctxt: ConfSerdeContext<'a>,
    pub _phantom_data: core::marker::PhantomData<fn() -> M>,
}

impl<'a, 'de, M> serde::de::DeserializeSeed<'de> for AsSeed<'a, M>
where
    M: InitializationStateMachine<'de> + From<ConfSerdeContext<'a>>,
{
    type Value = Result<M::Value, Vec<InnerError>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let AsSeed {
            struct_name,
            expecting_fn,
            ctxt,
            ..
        } = self;

        let doc_name = ctxt.document_name;
        let machine = M::from(ctxt);

        // This returns Result<Result, ...>> but it's always Ok because we map the serde error to a vec inner error.
        Ok(
            match deserializer.deserialize_struct(
                struct_name,
                machine.keys(),
                AsVisitor {
                    machine,
                    expecting_fn,
                },
            ) {
                Ok(machine) => machine.finalize(),
                Err(err) => Err(vec![InnerError::serde(doc_name, struct_name, err)]),
            },
        )
    }
}
