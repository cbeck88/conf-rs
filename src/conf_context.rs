use crate::{Error, ParsedEnv};
use clap::{parser::ValueSource, ArgMatches};

// Conf context stores everything that is needed to figure out if a user-defined
// program option in a (possibly flattened) struct was specified, and what string
// value to parse if so.
// It stores the results of CLI argument parsing, the env, and any prefixing
// that has been applied to the context so far.
// It provides getters which take care of the prefixing, aliases, etc.
// so that the generated code doesn't have to.
#[doc(hidden)]
pub struct ConfContext<'a> {
    args: &'a ArgMatches,
    env: &'a ParsedEnv,
    id_prefix: String,
}

impl<'a> ConfContext<'a> {
    pub(crate) fn new(args: &'a ArgMatches, env: &'a ParsedEnv) -> Result<Self, Error> {
        Ok(Self {
            args,
            env,
            id_prefix: String::default(),
        })
    }

    /// Check if a boolean program option was set to true, using any of its aliases or env value
    pub fn get_boolean_opt(&self, id: &str) -> Result<bool, Error> {
        Ok(self.args.get_flag(&(self.id_prefix.clone() + id)))
    }

    /// Get a string program option if it was set, using any of its aliases or env value
    /// Returns an error if it was set multiple times via args. If args and env are set, args shadows env.
    pub fn get_string_opt(&self, id: &str) -> Result<Option<(ValueSource, &'a str)>, Error> {
        let id = self.id_prefix.clone() + id;
        if let Some(val) = self.args.get_one::<String>(&id) {
            let value_source = self.args.value_source(&id).unwrap();
            return Ok(Some((value_source, val.as_str())));
        }

        Ok(None)
    }

    /// Get a repeat program option if it was set, using any of its aliases.
    /// If env is set, env is parsed via the delimiter (char).
    /// If args and env are set, args shadows env.
    pub fn get_repeat_opt(
        &self,
        id: &str,
        env_delimiter: Option<char>,
    ) -> Result<(ValueSource, Vec<&'a str>), Error> {
        let id = self.id_prefix.clone() + id;
        if let Some(val) = self.args.get_many::<String>(&id) {
            let value_source = self.args.value_source(&id).unwrap();

            let results: Vec<&'a str> = val.map(String::as_str).collect();

            if value_source == ValueSource::EnvVariable {
                // If value source is environment and we have an env delimiter, apply it
                if let Some(delim) = env_delimiter {
                    assert_eq!(results.len(), 1);
                    return Ok((value_source, results[0].split(delim).collect()));
                }
            }

            // FIXME: apply env delimiter
            Ok((value_source, results))
        } else {
            Ok((ValueSource::DefaultValue, vec![]))
        }
    }

    /// Create a new context from self, for use with a flattened substructure.
    ///
    /// Pass the id prefix that the substructure was configured with.
    /// That prefix will be concatenated to the prefixes already stored in this context.
    pub fn for_flattened(&self, sub_id_prefix: &str) -> ConfContext<'a> {
        ConfContext {
            args: self.args,
            env: self.env,
            id_prefix: self.id_prefix.clone() + sub_id_prefix,
        }
    }
}
