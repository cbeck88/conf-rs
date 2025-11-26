use conf::Conf;
use std::fmt;
use std::str::FromStr;

/// Type that implements Display but panics when formatted
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PanickingType(i32);

impl fmt::Display for PanickingType {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        panic!("PanickingType::fmt intentionally panics!");
    }
}

impl FromStr for PanickingType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<i32>()
            .map(PanickingType)
            .map_err(|e| e.to_string())
    }
}

/// Test that when default_help_str formatting panics, conf(test) catches it with should_panic.
/// This demonstrates that #[conf(test(should_panic))] works with Display panics during help rendering.
/// The panic occurs when parser_debug_asserts() calls render_clap_help(), which exercises the
/// default_help_str function pointer that calls Display::fmt on the default value.
#[derive(Conf)]
#[conf(test(should_panic))]
#[allow(dead_code)]
pub struct ConfigWithPanickingDefaultExpr {
    /// This field's default will panic when formatted for help text
    #[conf(long, default(PanickingType(42)))]
    value: PanickingType,
}

fn main() {}
