//! Shell completion via clap_complete.
//! Requires the Cargo "completion" feature to be enabled.

use crate::{Conf, ParsedEnv};
use clap::Command as ClapCommand;
use clap_complete::{aot::Shell as ClapShell, generate};
use std::{fmt, io, str::FromStr};

/// Shell names that provide autocompletion support
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Shell {
    /// Bourne Again shell (bash)
    Bash,
    /// Elvish shell
    Elvish,
    /// Friendly Interactive Shell (fish)
    Fish,
    /// PowerShell
    PowerShell,
    /// Z shell (zsh)
    Zsh,
}

impl Shell {
    #[inline]
    fn to_clap(self) -> ClapShell {
        match self {
            Shell::Bash => ClapShell::Bash,
            Shell::Elvish => ClapShell::Elvish,
            Shell::Fish => ClapShell::Fish,
            Shell::PowerShell => ClapShell::PowerShell,
            Shell::Zsh => ClapShell::Zsh,
        }
    }
}

#[derive(Debug, Clone)]
/// Error in case the specified shell name is not parsed correctly
pub struct ParseShellError {
    input: String,
}

impl fmt::Display for ParseShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid shell {:?} (expected: bash, elvish, fish, powershell, zsh)",
            self.input
        )
    }
}

impl std::error::Error for ParseShellError {}

impl FromStr for Shell {
    type Err = ParseShellError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = s.trim().to_ascii_lowercase();
        match v.as_str() {
            "bash" => Ok(Shell::Bash),
            "elvish" => Ok(Shell::Elvish),
            "fish" => Ok(Shell::Fish),
            "powershell" => Ok(Shell::PowerShell),
            "zsh" => Ok(Shell::Zsh),
            _ => Err(ParseShellError {
                input: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Shell::Bash => "bash",
            Shell::Elvish => "elvish",
            Shell::Fish => "fish",
            Shell::PowerShell => "powershell",
            Shell::Zsh => "zsh",
        })
    }
}

/// Internal: retrieve the clap::Command to give to clap_complete.
fn get_clap_command<C: Conf>() -> ClapCommand {
    let parsed_env = ParsedEnv::default();
    let program_options = <C as Conf>::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();

    let parser =
        <C as Conf>::get_parser(&parsed_env, program_options).expect("failed to build conf parser");

    parser.into_command()
}

/// Write completion script for `C` into `out`.
///
/// `bin_name` is the name used in the generated script; if `None`, we use the clap command name.
pub fn write_completion<C: Conf, W: std::io::Write>(
    shell: Shell,
    bin_name: Option<&str>,
    out: &mut W,
) -> std::io::Result<()> {
    let mut cmd = get_clap_command::<C>();

    let name: String = match bin_name {
        Some(s) => s.to_string(),
        None => cmd.get_name().to_string(),
    };

    generate(shell.to_clap(), &mut cmd, name, out);
    Ok(())
}

/// Generate completion script and return as bytes.
pub fn completion_bytes<C: Conf>(shell: Shell, bin_name: Option<&str>) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    write_completion::<C, _>(shell, bin_name, &mut buf)?;
    Ok(buf)
}

/// Generate completion script and return as a UTF-8 `String`.
pub fn completion_string<C: Conf>(shell: Shell, bin_name: Option<&str>) -> io::Result<String> {
    let bytes = completion_bytes::<C>(shell, bin_name)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
