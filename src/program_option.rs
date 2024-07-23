use crate::{CowStr, ParsedEnv};
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
    /// Any long-form switch aliases
    pub aliases: Vec<CowStr>,
    /// The env-form associated to this option, if any
    pub env_form: Option<CowStr>,
    /// Any env aliases
    pub env_aliases: Vec<CowStr>,
    /// The default-value, if any
    pub default_value: Option<CowStr>,
    /// Whether this option is considered required to appear. Affects help generation & semantics around flatten optional.
    pub is_required: bool,
    /// If set, tell clap to allow hyphen values. By default clap turns this off, which can help with error messages / misparses.
    pub allow_hyphen_values: bool,
    /// If set, then the user has specified that this is (or is not) a secret value, explicitly.
    pub secret: Option<bool>,
}

impl ProgramOption {
    /// Apply prefixing to a program option. This is done when it appears in a Conf structure that is then flattened
    /// into another one, and the flattening may have prefixes that need to be applied before the parser sees this program option.
    /// Note that prefixing does not apply to short forms, only long forms and env_forms.
    pub fn apply_flatten_prefixes(
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
            mut aliases,
            mut env_form,
            mut env_aliases,
            default_value,
            is_required,
            allow_hyphen_values,
            secret,
        } = self;

        id.to_mut().insert_str(0, id_prefix);

        if let Some(long_form) = long_form.as_mut() {
            if !long_prefix.is_empty() {
                long_form.to_mut().insert_str(0, long_prefix);
            }
        }
        for alias in aliases.iter_mut() {
            if !long_prefix.is_empty() {
                alias.to_mut().insert_str(0, long_prefix);
            }
        }
        if let Some(env_form) = env_form.as_mut() {
            if !env_prefix.is_empty() {
                env_form.to_mut().insert_str(0, env_prefix);
            }
        }
        for env_alias in env_aliases.iter_mut() {
            if !env_prefix.is_empty() {
                env_alias.to_mut().insert_str(0, env_prefix);
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
            aliases,
            env_form,
            env_aliases,
            default_value,
            is_required,
            allow_hyphen_values,
            secret,
        }
    }

    /// Drop our short form if it belongs to a list of forms to skip.
    /// This is applied when flattening if skip_short attribute is used.
    #[inline]
    pub fn skip_short_forms(mut self, skip_these: &[char], was_skipped: &mut [bool]) -> Self {
        if let Some(short) = self.short_form {
            if let Some(pos) = skip_these.iter().position(|to_skip| *to_skip == short) {
                self.short_form = None;
                was_skipped[pos] = true;
            }
        }
        self
    }

    /// Make this an "optional" option if it was previously required
    #[inline]
    pub fn make_optional(mut self) -> Self {
        self.is_required = false;
        self
    }

    /// Decide if a program option should be considered secret. Secrets need to be explicitly declared as such.
    #[inline]
    pub fn is_secret(&self) -> bool {
        self.secret.unwrap_or(false)
    }

    // Desired output is like:
    //  -x, --xyz <XYZ>
    //          This is the description.
    //          [env: XYZ=ABC]
    //          [default: 123]
    //
    // The env part is optional
    pub fn print(
        &self,
        stream: &mut impl std::fmt::Write,
        env: Option<&ParsedEnv>,
    ) -> Result<(), std::fmt::Error> {
        // Deal with spacing so that when short is 1 char, all the short options are aligned and indented, and all the long options are too.
        let dash = if self.short_form.is_some() { '-' } else { ' ' };
        let short = self.short_form.unwrap_or(' ');
        let comma = if self.short_form.is_some() && self.long_form.is_some() {
            ','
        } else {
            ' '
        };
        write!(stream, "  {dash}{short}{comma} ")?;
        if let Some(long) = self.long_form.as_ref() {
            write!(stream, "--{long} ")?;
        }
        if matches!(self.parse_type, ParseType::Parameter | ParseType::Repeat) {
            write!(stream, "<{}>", self.id)?;
        }
        writeln!(stream)?;
        if let Some(desc) = self.description.as_ref() {
            writeln!(stream, "          {}", desc.replace('\n', "\n          "))?;
        }
        if let Some(name) = self.env_form.as_deref() {
            if let Some(env) = env.filter(|_| !self.is_secret()) {
                let cur_val = env.get_lossy_or_default(name);
                writeln!(stream, "          [env: {name}={cur_val}]")?;
            } else {
                writeln!(stream, "          [env: {name}]")?;
            }
        }

        for name in &self.env_aliases {
            if let Some(env) = env.filter(|_| !self.is_secret()) {
                let cur_val = env.get_lossy_or_default(name);
                writeln!(stream, "          [env: {name}={cur_val}]")?;
            } else {
                writeln!(stream, "          [env: {name}]")?;
            }
        }

        if let Some(def) = self.default_value.as_ref() {
            writeln!(stream, "          [default: {def}]")?;
        }
        if self.is_secret() {
            writeln!(stream, "          [secret]")?;
        }
        Ok(())
    }
}
