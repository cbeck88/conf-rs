use conf::Conf;
use std::fmt;
use std::str::FromStr;

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

/// This test verifies that panics in Display::fmt propagate during help rendering.
/// The panic happens when parser_debug_asserts() calls render_clap_help(), which
/// exercises all default_help_str function pointers.
#[test]
#[should_panic(expected = "PanickingType::fmt intentionally panics!")]
fn test_display_panic_propagates_during_help_render() {
    #[derive(Conf)]
    #[allow(dead_code)]
    struct Config {
        #[conf(long, default(PanickingType(42)))]
        value: PanickingType,
    }

    // This should panic when rendering help, exercising the default_help_str function
    Config::parser_debug_asserts();
}

fn main() {}

/// Type that implements Display but returns an error
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ErrorType(i32);

impl fmt::Display for ErrorType {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }
}

impl FromStr for ErrorType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<i32>()
            .map(ErrorType)
            .map_err(|e| e.to_string())
    }
}

/// This test verifies that when Display::fmt returns Err, our custom panic message
/// includes the field name.
#[test]
#[should_panic(expected = "default_help_str function for option 'my_field' returned an error")]
fn test_display_error_includes_field_name() {
    #[derive(Conf)]
    #[allow(dead_code)]
    struct Config {
        #[conf(long, default(ErrorType(42)))]
        my_field: ErrorType,
    }

    // This should panic with our custom message including the field name
    Config::parser_debug_asserts();
}
