//! Shell completion via clap_complete.
//! Requires the Cargo "completion" feature to be enabled.
use crate::{Conf, ParsedEnv};

use clap::Command as ClapCommand;
pub use clap_complete::aot::Shell;
use clap_complete::generate;

use std::io;

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

    generate(shell, &mut cmd, name, out);
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
