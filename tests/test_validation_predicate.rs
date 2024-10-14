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

#[derive(Conf, Debug)]
#[conf(validation_predicate = MultiConstraint::b_required_if, validation_predicate = MultiConstraint::c_required_if)]
struct MultiConstraint {
    #[arg(short)]
    a: Option<String>,
    #[arg(short)]
    b: Option<String>,
    #[arg(short)]
    c: Option<String>,
}

impl MultiConstraint {
    fn b_required_if(&self) -> Result<(), &'static str> {
        if self.a == Some("b".to_owned()) && self.b.is_none() {
            return Err("b is required if a = 'b'");
        }
        Ok(())
    }

    fn c_required_if(&self) -> Result<(), &'static str> {
        if self.b == Some("c".to_owned()) && self.c.is_none() {
            return Err("c is required if b = 'c'");
        }
        Ok(())
    }
}

#[test]
fn test_multiple_validate_predicates() {
    let result = MultiConstraint::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert_eq!(result.a, None);
    assert_eq!(result.b, None);
    assert_eq!(result.c, None);

    let result =
        MultiConstraint::try_parse_from::<&str, &str, &str>(vec![".", "-a", "x"], vec![]).unwrap();
    assert_eq!(result.a, Some("x".to_owned()));
    assert_eq!(result.b, None);
    assert_eq!(result.c, None);

    assert_error_contains_text!(
        MultiConstraint::try_parse_from::<&str, &str, &str>(vec![".", "-a", "b"], vec![]),
        ["b is required if a = 'b'"]
    );

    let result = MultiConstraint::try_parse_from::<&str, &str, &str>(
        vec![".", "-a", "b", "-b", "x"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.a, Some("b".to_owned()));
    assert_eq!(result.b, Some("x".to_owned()));
    assert_eq!(result.c, None);

    assert_error_contains_text!(
        MultiConstraint::try_parse_from::<&str, &str, &str>(
            vec![".", "-a", "b", "-b", "c"],
            vec![]
        ),
        ["c is required if b = 'c'"]
    );

    let result = MultiConstraint::try_parse_from::<&str, &str, &str>(
        vec![".", "-a", "b", "-b", "c", "-c", "x"],
        vec![],
    )
    .unwrap();
    assert_eq!(result.a, Some("b".to_owned()));
    assert_eq!(result.b, Some("c".to_owned()));
    assert_eq!(result.c, Some("x".to_owned()));
}
