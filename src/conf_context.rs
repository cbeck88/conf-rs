use crate::{str_to_bool, InnerError, ParseType, ParsedArgs, ParsedEnv, ProgramOption};
use clap::parser::ValueSource;
use core::fmt::Debug;

// Data about the source of a value returned by ConfContext functions
// This is mainly used to render help if something fails in the value parser later
// It is generic over a string type so that it can accommodate owned and borrowed data.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ConfValueSource<S>
where
    S: Clone + Debug + Eq + PartialEq + Ord + PartialOrd,
{
    Args,
    Env(S),
    Default,
}

impl<'a> ConfValueSource<&'a str> {
    pub fn into_owned(self) -> ConfValueSource<String> {
        match self {
            Self::Args => ConfValueSource::Args,
            Self::Env(s) => ConfValueSource::Env(s.to_owned()),
            Self::Default => ConfValueSource::Default,
        }
    }
}

impl<'a> From<ValueSource> for ConfValueSource<&'a str> {
    fn from(src: ValueSource) -> Self {
        match &src {
            ValueSource::CommandLine => Self::Args,
            ValueSource::DefaultValue => Self::Default,
            ValueSource::EnvVariable => unreachable!("clap should not be parsing env here"),
            _ => panic!("this is an unknown value source from clap: {src:?}"),
        }
    }
}

// Data stored when we start parsing a flattened-optional field.
// This is used in error messages about why a field became required.
#[doc(hidden)]
#[derive(Clone, Debug)]
pub(crate) struct FlattenedOptionalDebugInfo<'a> {
    pub struct_name: &'static str,
    pub id_prefix: String,
    pub option_appeared: &'a ProgramOption,
    pub value_source: ConfValueSource<&'a str>,
}

// Conf context stores everything that is needed to figure out if a user-defined
// program option in a (possibly flattened) struct was specified, and what string
// value to parse if so.
// It stores the results of CLI argument parsing, the env, and any prefixing
// that has been applied to the context so far.
// It provides getters which take care of the prefixing, aliases, etc.
// so that the generated code doesn't have to.
//
// Many of the APIs which take an id (or list of ids) will panic if the id is not found.
// This is okay because this is not a user facing object, and it's okay to panic for internal logic errors like that.
#[doc(hidden)]
pub struct ConfContext<'a> {
    args: &'a ParsedArgs<'a>,
    env: &'a ParsedEnv,
    id_prefix: String,
    flattened_optional_debug_info: Option<FlattenedOptionalDebugInfo<'a>>,
}

impl<'a> ConfContext<'a> {
    pub(crate) fn new(args: &'a ParsedArgs<'a>, env: &'a ParsedEnv) -> Self {
        Self {
            args,
            env,
            id_prefix: String::default(),
            flattened_optional_debug_info: None,
        }
    }

    fn get_env(
        &self,
        env_name: &'a str,
        opt: &'a ProgramOption,
    ) -> Result<Option<&'a str>, InnerError> {
        if let Some(val) = self.env.get(env_name) {
            return Ok(Some(val.to_str().ok_or_else(|| {
                if opt.is_secret() {
                    InnerError::invalid_utf8_env(env_name, opt, None)
                } else {
                    InnerError::invalid_utf8_env(env_name, opt, Some(val))
                }
            })?));
        }

        Ok(None)
    }

    /// Check if a boolean program option was set to true, using any of its aliases or env value
    pub fn get_boolean_opt(
        &self,
        id: &str,
    ) -> Result<(ConfValueSource<&'a str>, bool), InnerError> {
        let id = self.id_prefix.clone() + id;
        if self.args.arg_matches.get_flag(&id) {
            return Ok((ConfValueSource::<&'a str>::Args, true));
        }
        let opt = self.args.id_to_option.get(id.as_str()).unwrap_or_else(|| {
            panic!(
                "Option not found by id ({id}), this is an internal_error: {:?}",
                self.args.id_to_option
            )
        });
        if let Some(env_form) = opt.env_form.as_deref() {
            if let Some(val) = self.get_env(env_form, opt)? {
                return Ok((ConfValueSource::<&'a str>::Env(env_form), str_to_bool(val)));
            }
        }

        Ok((ConfValueSource::Default, false))
    }

    /// Get a string program option if it was set, using any of its aliases or env value
    /// Returns an error if it was set multiple times via args. If args and env are set, args shadows env.
    #[allow(clippy::type_complexity)]
    pub fn get_string_opt(
        &self,
        id: &str,
    ) -> Result<
        (
            Option<(ConfValueSource<&'a str>, &'a str)>,
            &'a ProgramOption,
        ),
        InnerError,
    > {
        let id = self.id_prefix.clone() + id;
        let opt = self.args.id_to_option.get(id.as_str()).unwrap_or_else(|| {
            panic!(
                "Option not found by id ({id}), this is an internal_error: {:?}",
                self.args.id_to_option
            )
        });
        if opt.short_form.is_some() || opt.long_form.is_some() {
            if let Some(val) = self.args.arg_matches.get_one::<String>(&id) {
                let value_source = self
                    .args
                    .arg_matches
                    .value_source(&id)
                    .expect("Id not found, this is an internal error");
                // Note: We don't support user-defined default value on this one right now, and we don't give default values to clap so this should be the only possibility
                assert_eq!(value_source, ValueSource::CommandLine);

                let val_and_source = Some((value_source.into(), val.as_str()));
                // Args take precedence over env, so return now if we got args.
                // If we got default_value we want to fall through to env
                return Ok((val_and_source, opt));
            }
        }

        if let Some(env_form) = opt.env_form.as_deref() {
            if let Some(val) = self.get_env(env_form, opt)? {
                return Ok((Some((ConfValueSource::<&str>::Env(env_form), val)), opt));
            }
        }

        for env_alias in opt.env_aliases.iter() {
            if let Some(val) = self.get_env(env_alias, opt)? {
                let value_source = ConfValueSource::<&str>::Env(env_alias);
                let val_and_source = Some((value_source, val));

                return Ok((val_and_source, opt));
            }
        }

        if let Some(default_val) = opt.default_value.as_deref() {
            let value_source = ConfValueSource::Default;
            let val_and_source = Some((value_source, default_val));

            return Ok((val_and_source, opt));
        }

        Ok((None, opt))
    }

    /// Get a repeat program option if it was set, using any of its aliases.
    /// If env is set, env is parsed via the delimiter (char).
    /// If args and env are set, args shadows env.
    pub fn get_repeat_opt(
        &self,
        id: &str,
        env_delimiter: Option<char>,
    ) -> Result<(ConfValueSource<&'a str>, Vec<&'a str>, &'a ProgramOption), InnerError> {
        let id = self.id_prefix.clone() + id;
        let opt = self.args.id_to_option.get(id.as_str()).unwrap_or_else(|| {
            panic!(
                "Option not found by id ({id}), this is an internal_error: {:?}",
                self.args.id_to_option
            )
        });

        if let Some(val) = self.args.arg_matches.get_many::<String>(&id) {
            let value_source = self
                .args
                .arg_matches
                .value_source(&id)
                .expect("Id not found, this is an internal error");
            // Note: We don't support user-defined default value on this one right now, and we don't give default values to clap so this should be the only possibility
            assert_eq!(value_source, ValueSource::CommandLine);

            let results: Vec<&'a str> = val.map(String::as_str).collect();

            return Ok((value_source.into(), results, opt));
        }

        if let Some(env_form) = opt.env_form.as_deref() {
            if let Some(val) = self.get_env(env_form, opt)? {
                let value_source = ConfValueSource::<&str>::Env(env_form);

                return Ok(if let Some(delim) = env_delimiter {
                    (value_source, val.split(delim).collect(), opt)
                } else {
                    (value_source, vec![val], opt)
                });
            }
        }

        for env_alias in opt.env_aliases.iter() {
            if let Some(val) = self.get_env(env_alias, opt)? {
                let value_source = ConfValueSource::<&str>::Env(env_alias);

                return Ok(if let Some(delim) = env_delimiter {
                    (value_source, val.split(delim).collect(), opt)
                } else {
                    (value_source, vec![val], opt)
                });
            }
        }

        Ok((ValueSource::DefaultValue.into(), vec![], opt))
    }

    /// Check if a given option appears in cli args or env (not defaulted)
    /// This is used to implement any_program_options_appeared which supports flatten-optional
    pub fn option_appears(&self, id: &str) -> Result<Option<ConfValueSource<&'a str>>, InnerError> {
        if let Some(value_source) = self.get_value_source(id)? {
            Ok(match value_source {
                ConfValueSource::Default => None,
                other => Some(other),
            })
        } else {
            Ok(None)
        }
    }

    /// Returns the value source of a given program option id (relative to our prefix), if it has a value
    fn get_value_source(&self, id: &str) -> Result<Option<ConfValueSource<&'a str>>, InnerError> {
        let prefixed_id = self.id_prefix.clone() + id;
        let opt = self
            .args
            .id_to_option
            .get(prefixed_id.as_str())
            .unwrap_or_else(|| {
                panic!(
                    "Option not found by id ({prefixed_id}), this is an internal_error: {:?}",
                    self.args.id_to_option
                )
            });

        Ok(match opt.parse_type {
            ParseType::Flag => {
                let (src, _val) = self.get_boolean_opt(id)?;
                Some(src)
            }
            ParseType::Parameter => {
                let (maybe, _opt) = self.get_string_opt(id)?;
                maybe.map(|(src, _val)| src)
            }
            ParseType::Repeat => {
                // Hack: don't supply delimiter char even if it exists, since it won't matter for this function
                let (src, _val, _opt) = self.get_repeat_opt(id, None)?;
                Some(src)
            }
        })
    }

    /// Create a new context from self, for use with a flattened substructure.
    ///
    /// Pass the id prefix that the substructure was configured with.
    /// That prefix will be concatenated to the prefixes already stored in this context.
    #[inline]
    pub fn for_flattened(&self, sub_id_prefix: &str) -> ConfContext<'a> {
        ConfContext {
            args: self.args,
            env: self.env,
            id_prefix: self.id_prefix.clone() + sub_id_prefix,
            flattened_optional_debug_info: self.flattened_optional_debug_info.clone(),
        }
    }

    /// Create a new context from self, for use with a flattened-optional substructure.
    ///
    /// This preserves context about what optional group we are entering, and why it was enabled, for error messages.
    #[inline]
    pub fn for_flattened_optional(
        &self,
        sub_id_prefix: &str,
        struct_name: &'static str,
        option_appeared_result: (&str, ConfValueSource<&'a str>),
    ) -> ConfContext<'a> {
        let id_prefix = self.id_prefix.clone() + sub_id_prefix;
        let (option_appeared_relative_id, value_source) = option_appeared_result;
        let prefixed_id = id_prefix.clone() + option_appeared_relative_id;

        let option_appeared = *self
            .args
            .id_to_option
            .get(prefixed_id.as_str())
            .unwrap_or_else(|| panic!("Option not found by id ({prefixed_id}), option_appeared_relative_id = {option_appeared_relative_id}, this is an internal_error: {:?}", self.args.id_to_option));

        let flattened_optional_debug_info = Some(FlattenedOptionalDebugInfo {
            struct_name,
            id_prefix: id_prefix.clone(),
            option_appeared,
            value_source,
        });

        ConfContext {
            args: self.args,
            env: self.env,
            id_prefix,
            flattened_optional_debug_info,
        }
    }

    /// Get the id prefix of this conf context
    pub fn get_id_prefix(&self) -> &str {
        &self.id_prefix
    }

    /// Generate a "missing_required_parameter" error
    ///
    /// This error includes context if we are within a flattened optional group
    pub fn missing_required_parameter_error(&self, opt: &ProgramOption) -> InnerError {
        InnerError::missing_required_parameter(opt, self.flattened_optional_debug_info.clone())
    }

    /// Generate a "too_few_arguments" error
    ///
    /// This should contain the ids of all "single options" in this constraint, as well as all flattened options in this constraint.
    /// This error includes context if we are within a flattened optional group
    pub fn too_few_arguments_error(
        &self,
        struct_name: &'static str,
        constraint_single_option_ids: &[&str],
        constraint_flattened_ids: &[&str],
    ) -> InnerError {
        let single_options = constraint_single_option_ids.iter().map(|id| {
            let prefixed_id = self.id_prefix.clone() + id;
            *self
                .args
                .id_to_option
                .get(prefixed_id.as_str())
                .unwrap_or_else(|| panic!("Option not found by id ({prefixed_id}), this is an internal_error: {:?}", self.args.id_to_option))      
        }).collect::<Vec<&ProgramOption>>();
        InnerError::too_few_arguments(
            struct_name,
            &self.id_prefix,
            single_options,
            constraint_flattened_ids,
            self.flattened_optional_debug_info.clone(),
        )
    }

    /// Generate a "too_many_arguments" error
    ///
    /// To use this function correctly, constraint_single_option_ids can be the relative id of any field in the constraint. (So, it's field name on the current struct.)
    /// Whether or not that one actually appeared and is contributing to the error, `ConfContext` will figure that out, and filter out any that shouldn't be in the error report.
    ///
    /// constraint_flattened_ids should include the id of the flattened optional struct contributing to the error, and the result of Conf::any_program_options_appeared on the inner struct.
    /// This includes enough detail to render an error for at least one option that enabled that flattened optional struct. If the result is None then conf context will filter that out, so that
    /// the proc macro doesn't have to.
    #[allow(clippy::type_complexity)]
    pub fn too_many_arguments_error(
        &self,
        struct_name: &'static str,
        constraint_single_option_ids: &[&str],
        constraint_flattened_ids: Vec<(&str, Option<(&str, ConfValueSource<&'a str>)>)>,
    ) -> InnerError {
        let single_options = constraint_single_option_ids.iter().filter_map(|id| {
            let prefixed_id = self.id_prefix.clone() + id;
            let opt = *self
                .args
                .id_to_option
                .get(prefixed_id.as_str())
                .unwrap_or_else(|| panic!("Option not found by id ({prefixed_id}), this is an internal_error: {:?}", self.args.id_to_option));
            self.get_value_source(id).expect("internal error").and_then(|value_source| {
                if matches!(&value_source, ConfValueSource::Default) {
                    None
                } else {
                    Some((opt, value_source))
                }
            })
        }).collect::<Vec<(&ProgramOption, ConfValueSource<&'a str>)>>();

        let flattened_options = constraint_flattened_ids.into_iter().filter_map(|(flattened_field, maybe_appearing_option)| {
            maybe_appearing_option.map(|(id, value_source)| {
                let absolute_id = self.id_prefix.clone() + flattened_field + "." + id;
                let opt = self
                    .args
                    .id_to_option
                    .get(absolute_id.as_str())
                    .unwrap_or_else(|| panic!("Option not found by id ({absolute_id}), this is an internal_error: {:?}", self.args.id_to_option));
                (flattened_field, *opt, value_source)
            })
        }).collect::<Vec<(&str, &ProgramOption, ConfValueSource<&'a str>)>>();

        InnerError::too_many_arguments(
            struct_name,
            &self.id_prefix,
            single_options,
            flattened_options,
        )
    }
}
