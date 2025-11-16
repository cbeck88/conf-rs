//! Terminal styling definitions for help text output.

/// Terminal styling definitions for help text output.
///
/// This is a newtype wrapper around `clap::builder::Styles` to avoid exposing clap types
/// directly in the public API. Use this with the `#[conf(styles = ...)]` attribute to
/// customize the colors and formatting of help text.
///
/// # Example
///
/// ```rust
/// use conf::Conf;
/// use anstyle::{AnsiColor, Style};
///
/// const HELP_STYLES: conf::Styles = conf::Styles::styled()
///     .header(AnsiColor::Blue.on_default().bold())
///     .usage(AnsiColor::Blue.on_default().bold())
///     .literal(AnsiColor::White.on_default())
///     .placeholder(AnsiColor::Green.on_default());
///
/// #[derive(Conf)]
/// #[conf(styles = HELP_STYLES)]
/// struct Args {
///     #[conf(long)]
///     name: String,
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Styles(clap::builder::Styles);

impl Styles {
    /// Create styles with no terminal coloring.
    pub const fn plain() -> Self {
        Self(clap::builder::Styles::plain())
    }

    /// Create default terminal styling with colors enabled.
    pub const fn styled() -> Self {
        Self(clap::builder::Styles::styled())
    }

    /// Style for general headings.
    pub const fn header(mut self, style: anstyle::Style) -> Self {
        self.0 = self.0.header(style);
        self
    }

    /// Style for error messages.
    pub const fn error(mut self, style: anstyle::Style) -> Self {
        self.0 = self.0.error(style);
        self
    }

    /// Style for usage headings.
    pub const fn usage(mut self, style: anstyle::Style) -> Self {
        self.0 = self.0.usage(style);
        self
    }

    /// Style for command syntax (e.g., `--help`).
    pub const fn literal(mut self, style: anstyle::Style) -> Self {
        self.0 = self.0.literal(style);
        self
    }

    /// Style for value names within syntax (e.g., `<FILE>`).
    pub const fn placeholder(mut self, style: anstyle::Style) -> Self {
        self.0 = self.0.placeholder(style);
        self
    }

    /// Style for suggested correct usage.
    pub const fn valid(mut self, style: anstyle::Style) -> Self {
        self.0 = self.0.valid(style);
        self
    }

    /// Style for incorrect usage.
    pub const fn invalid(mut self, style: anstyle::Style) -> Self {
        self.0 = self.0.invalid(style);
        self
    }

    /// Get the inner clap Styles (for internal use).
    #[doc(hidden)]
    pub const fn into_clap_styles(self) -> clap::builder::Styles {
        self.0
    }
}
