mod common;
use common::*;

use conf::Conf;

#[derive(Conf, Debug)]
#[conf(validation_predicate = TwoOf::validate)]
struct TwoOf {
    #[arg(short)]
    a: bool,
    #[arg(short)]
    b: bool,
    #[arg(short)]
    c: bool,
}

impl TwoOf {
    fn validate(&self) -> Result<(), &'static str> {
        let num = self.a as u32 + self.b as u32 + self.c as u32;
        match num {
            0 | 1 => Err("Too few flags set"),
            2 => Ok(()),
            _ => Err("Too many flags set"),
        }
    }
}

#[test]
fn test_validate_predicate_two_of_parsing() {
    assert_error_contains_text!(
        TwoOf::try_parse_from::<&str, &str, &str>(vec!["."], vec![]),
        ["Too few flags set"]
    );
    assert_error_contains_text!(
        TwoOf::try_parse_from::<&str, &str, &str>(vec![".", "-a"], vec![]),
        ["Too few flags set"]
    );
    assert_error_contains_text!(
        TwoOf::try_parse_from::<&str, &str, &str>(vec![".", "-b"], vec![]),
        ["Too few flags set"]
    );
    assert_error_contains_text!(
        TwoOf::try_parse_from::<&str, &str, &str>(vec![".", "-c"], vec![]),
        ["Too few flags set"]
    );
    assert_error_contains_text!(
        TwoOf::try_parse_from::<&str, &str, &str>(vec![".", "-a", "-b", "-c"], vec![]),
        ["Too many flags set"]
    );

    let result = TwoOf::try_parse_from::<&str, &str, &str>(vec![".", "-a", "-b"], vec![]).unwrap();
    assert!(result.a);
    assert!(result.b);
    assert!(!result.c);

    let result = TwoOf::try_parse_from::<&str, &str, &str>(vec![".", "-a", "-c"], vec![]).unwrap();
    assert!(result.a);
    assert!(!result.b);
    assert!(result.c);

    let result = TwoOf::try_parse_from::<&str, &str, &str>(vec![".", "-c", "-b"], vec![]).unwrap();
    assert!(!result.a);
    assert!(result.b);
    assert!(result.c);
}
