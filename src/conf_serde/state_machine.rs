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
///
/// NOTE: The API was modified to take reference to context
///
/// The context could be stored in the machine generally, and this trait would be simpler. But it actually leads
/// to more complex code-gen on the proc-macro side, because it makes it harder to work with the current implementation
/// of the non-serde side of things, and so we push some complexity onto this trait instead for now.
#[doc(hidden)]
pub trait InitializationStateMachine<'de>: Sized {
    type Value;
    type Context<'c>;

    fn keys() -> &'static [&'static str];
    fn next<'c, NVP>(
        self,
        key: &str,
        next_value_producer: NVP,
        context: &Self::Context<'c>,
    ) -> Self
    where
        NVP: NextValueProducer<'de>;
    fn finalize<'c>(self, context: &Self::Context<'c>) -> Result<Self::Value, Vec<InnerError>>;
}

/// Wrapper type to treat an initialization state machine as a serde::de::Visitor impl.
/// This is only here to avoid using blanket implementations, which can become problematic.
///
/// An ISM can be used in phases, but it can also obviously be used in one shot from a deserializer.
/// That's used when implementing DeserializeSeed on a struct.
#[doc(hidden)]
pub struct AsVisitor<'c, 'de, M>
where
    M: InitializationStateMachine<'de>,
{
    pub machine: M,
    pub ctxt: M::Context<'c>,
    pub expecting_fn: fn(&mut fmt::Formatter) -> fmt::Result,
}

impl<'c, 'de, M> serde::de::Visitor<'de> for AsVisitor<'c, 'de, M>
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
        let Self {
            ctxt, mut machine, ..
        } = self;

        while let Some(key) = map_access.next_key::<IdentString>()? {
            machine = machine.next(key.as_str(), &mut map_access, &ctxt);
        }
        Ok(machine.finalize(&ctxt))
    }
}

/// The details of DeserializeSeed::deserialize don't really need to be code-genned,
/// and it's simpler to move code out of the proc macro when possible.
///
/// But because of orphan rules, the Seed can't actually appear in the conf crate,
/// it has to be in the user's crate.
///
/// So the `struct Seed` is code-genned and the `impl DeserializeSeed` is a thin
/// stub that calls right to this.
#[doc(hidden)]
pub fn deserialize_seed_impl<'a, 'de, D, M>(
    struct_name: &'static str,
    expecting_fn: fn(&mut fmt::Formatter) -> fmt::Result,
    ctxt: ConfSerdeContext<'a>,
    deserializer: D,
) -> Result<M::Value, Vec<InnerError>>
where
    M: InitializationStateMachine<'de, Context<'a> = ConfSerdeContext<'a>> + Default,
    D: serde::de::Deserializer<'de>,
{
    let doc_name = ctxt.document_name;
    let machine = M::default();

    match deserializer.deserialize_struct(
        struct_name,
        M::keys(),
        AsVisitor {
            machine,
            ctxt,
            expecting_fn,
        },
    ) {
        Ok(result) => result,
        Err(err) => Err(vec![InnerError::serde(doc_name, struct_name, err)]),
    }
}
