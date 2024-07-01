use crate::{str_to_bool, Error, InnerError, ParsedArgs, ParsedEnv};
use std::fmt;

// Conf context stores everything that is needed to figure out if a user-defined
// program option in a (possibly flattened) struct was specified, and what string
// value to parse if so.
// It stores the results of CLI argument parsing, the env, and any prefixing
// that has been applied to the context so far.
// It provides getters which take care of the prefixing, aliases, etc.
// so that the generated code doesn't have to.
#[doc(hidden)]
pub struct ConfContext<'a> {
    args: &'a ParsedArgs,
    env: &'a ParsedEnv,
    long_prefix: String,
    env_prefix: String,
}

// Value-source keeps track of where a particular string value came from,
// for better error reporting if value_parser fails subsequently
#[doc(hidden)]
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ValueSource {
    ShortFlag(String),
    LongFlag(String),
    EnvVar(String),
    Default,
}

impl fmt::Display for ValueSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ShortFlag(name) => write!(f, "'-{name}'"),
            Self::LongFlag(name) => write!(f, "'--{name}'"),
            Self::EnvVar(name) => write!(f, "environment variable '{name}'"),
            Self::Default => write!(f, "default"),
        }
    }
}

impl<'a> ConfContext<'a> {
    pub(crate) fn new(args: &'a ParsedArgs, env: &'a ParsedEnv) -> Result<Self, Error> {
        Ok(Self {
            args,
            env,
            long_prefix: String::default(),
            env_prefix: String::default(),
        })
    }

    /// Check if a short flag was set in this context
    fn has_short_flag(&self, name: &str) -> bool {
        self.args.has_short_flag(name)
    }

    /// Check if a long flag was set in this context
    fn has_long_flag(&self, name: &str) -> bool {
        let mut prefixed = self.long_prefix.clone();
        prefixed.push_str(name);
        self.args.has_long_flag(&prefixed)
    }

    /// Get a short parameter value if present in this context
    fn get_short_parameter(&self, name: &str) -> Option<(ValueSource, &'a str)> {
        self.args
            .get_short_parameter(name)
            .map(|val| (ValueSource::ShortFlag(name.to_owned()), val))
    }

    /// Get a long parameter value if present in this context
    fn get_long_parameter(&self, name: &str) -> Option<(ValueSource, &'a str)> {
        let mut prefixed = self.long_prefix.clone();
        prefixed.push_str(name);
        self.args
            .get_long_parameter(&prefixed)
            .map(|val| (ValueSource::LongFlag(prefixed), val))
    }

    /// Get a repeated option value. Slice will be empty if not present in this context.
    fn get_repeat(&self, name: &str) -> (ValueSource, &'a [String]) {
        let mut prefixed = self.long_prefix.clone();
        prefixed.push_str(name);
        let list = self.args.get_repeat(&prefixed);
        (ValueSource::LongFlag(prefixed), list)
    }

    /// Get an env value if present in this context
    fn get_env(&self, name: &str) -> Result<Option<(ValueSource, &'a str)>, Error> {
        let mut prefixed = self.env_prefix.clone();
        prefixed.push_str(name);
        if let Some(os_str) = self.env.get(&prefixed) {
            let val = os_str
                .to_str()
                .ok_or_else(|| InnerError::InvalidUtf8Env(prefixed.clone()))?;
            Ok(Some((ValueSource::EnvVar(prefixed), val)))
        } else {
            Ok(None)
        }
    }

    /// Check if a boolean program option was set to true, using any of its aliases or env value
    pub fn get_boolean_opt(
        &self,
        short_name: Option<&str>,
        long_name: Option<&str>,
        env_name: Option<&str>,
    ) -> Result<bool, Error> {
        if let Some(name) = short_name {
            if self.has_short_flag(name) {
                return Ok(true);
            }
        }
        if let Some(name) = long_name {
            if self.has_long_flag(name) {
                return Ok(true);
            }
        }
        Ok(if let Some(name) = env_name {
            self.get_env(name)?
                .map(|(_value_source, val)| str_to_bool(val))
                .unwrap_or(false)
        } else {
            false
        })
    }

    /// Get a string program option if it was set, using any of its aliases or env value
    /// Returns an error if it was set multiple times via args. If args and env are set, args shadows env.
    pub fn get_string_opt(
        &self,
        short_name: Option<&str>,
        long_name: Option<&str>,
        env_name: Option<&str>,
    ) -> Result<Option<(ValueSource, &'a str)>, Error> {
        let mut result: Option<(ValueSource, &str)> = None;

        if let Some(name) = short_name {
            if let Some((source, val)) = self.get_short_parameter(name) {
                if let Some((prev_source, _val)) = result {
                    return Err(InnerError::ProgramOptionSpecifiedTwice(source, prev_source).into());
                }
                result = Some((source, val));
            }
        }

        if let Some(name) = long_name {
            if let Some((source, val)) = self.get_long_parameter(name) {
                if let Some((prev_source, _val)) = result {
                    return Err(InnerError::ProgramOptionSpecifiedTwice(source, prev_source).into());
                }
                result = Some((source, val));
            }
        }

        if result.is_some() {
            return Ok(result);
        }

        if let Some(name) = env_name {
            self.get_env(name)
        } else {
            Ok(None)
        }
    }

    /// Get a repeat program option if it was set, using any of its aliases.
    /// If env is set, env is parsed via the delimiter (char).
    /// If args and env are set, args shadows env.
    pub fn get_repeat_opt(
        &self,
        long_name: Option<&str>,
        env: Option<(&str, Option<char>)>,
    ) -> Result<(ValueSource, Vec<&'a str>), Error> {
        let mut result = vec![];

        if let Some(name) = long_name {
            let (value_source, slice) = self.get_repeat(name);
            result.extend(slice.iter().map(String::as_str));
            if !(result.is_empty()) {
                return Ok((value_source, result));
            }
        }

        if let Some((name, maybe_delimiter)) = env {
            if let Some((value_source, str_val)) = self.get_env(name)? {
                if let Some(delimiter) = maybe_delimiter {
                    result.extend(str_val.split(delimiter));
                } else {
                    result.push(str_val);
                }
                return Ok((value_source, result));
            }
        }

        Ok((ValueSource::Default, result))
    }

    /// Create a new context from self, for use with a flattened substructure.
    ///
    /// Pass the switch prefix and env prefix that the substructure was configured with.
    /// Those prefixes will be added to the prefixes already stored in this context.
    pub fn for_flattened(&self, sub_long_prefix: &str, sub_env_prefix: &str) -> ConfContext<'a> {
        let mut long_prefix = self.long_prefix.clone();
        long_prefix.push_str(sub_long_prefix);

        let mut env_prefix = self.env_prefix.clone();
        env_prefix.push_str(sub_env_prefix);

        ConfContext {
            args: self.args,
            env: self.env,
            long_prefix,
            env_prefix,
        }
    }
}
