use crate::CowStr;
use std::fmt;

/// This is a property of every program option, and dictates what form of data we expect to collect from CLI and env.
/// This also affects the parser's expectations when it encounters a switch associated to this program option -- does
/// it expect to associate the next argument with this parameter? And it classifies the results of its parsing based
/// on the parse-type of the switch.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ParseType {
    /// A flag is a switch which appears and has no arguments, it is either present or absent
    Flag,
    /// A parameter is a switch which appears and has one expected argument.
    Parameter,
    /// A repeat parameter is a switch which may appear one or more times, each time supplying an argument.
    Repeat,
}

impl fmt::Display for ParseType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Flag => write!(f, "Flag"),
            Self::Parameter => write!(f, "Parameter"),
            Self::Repeat => write!(f, "Repeat"),
        }
    }
}

/// Description of a program option, sufficient to identify it on command line or in env, and to render help text for it
/// It may have one long form and one short form
#[doc(hidden)]
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct ProgramOption {
    /// Id of this option. This is typically the field name literal, and on flattening we prepend it with `parent.`
    pub id: CowStr,
    /// Parse type of this option
    pub parse_type: ParseType,
    /// Description (typically its doc string)
    pub description: Option<CowStr>,
    /// The short-form switch (-) associated to this option, if any
    pub short_form: Option<char>,
    /// The long-form switch (--) associated to this option, if any
    pub long_form: Option<CowStr>,
    /// The env-form associated to this option, if any
    pub env_form: Option<CowStr>,
    /// The default-value, if any
    pub default_value: Option<CowStr>,
    /// Whether this option is considered required to appear. Affects help generation
    pub is_required: bool,
}

impl ProgramOption {
    /// Apply prefixing to a program option. This is done when it appears in a Conf structure that is then flattened
    /// into another one, and the flattening may have prefixes that need to be applied before the parser sees this program option.
    /// Note that prefixing does not apply to short forms, only long forms and env_forms.
    pub fn flatten(
        self,
        id_prefix: &str,
        long_prefix: &str,
        env_prefix: &str,
        description_prefix: &str,
    ) -> ProgramOption {
        let ProgramOption {
            mut id,
            parse_type,
            mut description,
            short_form,
            mut long_form,
            mut env_form,
            default_value,
            is_required,
        } = self;

        id.to_mut().insert_str(0, id_prefix);

        if let Some(long_form) = long_form.as_mut() {
            if !long_prefix.is_empty() {
                long_form.to_mut().insert_str(0, long_prefix);
            }
        }
        if let Some(env_form) = env_form.as_mut() {
            if !env_prefix.is_empty() {
                env_form.to_mut().insert_str(0, env_prefix);
            }
        }
        if let Some(desc) = description.as_mut() {
            // Description prefix requires a little more subtlety to try to ensure that it is going to be readable,
            // because typically we trim all the doc strings of leading and trailing whitespace, but retain the line breaks.
            // The description prefix is usually similarly trimmed. But there should be some whitespace between the prefix and description
            // if this is human-readable text.
            //
            // To decide what to do, we look at both the prefix and the description. If either of them has newlines, then we join with a newline.
            // Otherwise we join with a space. If the prefix is empty string, then we don't join with anything.
            //
            // Probably won't work well in all cases, but it's a start.
            // In the future, we should probably give the user more control, like, if they pass `help_format` instead of `help_prefix`, then
            // we assume the doc string is a formatting string or something like this, and do like displaydoc does.
            if !description_prefix.is_empty() {
                let desc = desc.to_mut();
                let ws = if description_prefix.contains('\n') || desc.contains('\n') {
                    '\n'
                } else {
                    ' '
                };
                desc.insert(0, ws);
                desc.insert_str(0, description_prefix);
            }
        }

        ProgramOption {
            id,
            parse_type,
            description,
            short_form,
            long_form,
            env_form,
            default_value,
            is_required,
        }
    }
}
