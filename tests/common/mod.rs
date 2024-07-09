#![allow(unused, dead_code)]

use conf::Error;
use core::cmp::min;

// Helper for making Vec<String> concisely
pub fn vec_str(list: impl IntoIterator<Item = &'static str>) -> Vec<String> {
    list.into_iter().map(Into::into).collect()
}

#[macro_export]
macro_rules! assert_multiline_eq {
    ($left:expr, $right:expr) => {
        assert_multiline_eq($left, $right, file!(), line!(), column!())
    };
}

pub fn assert_multiline_eq(left: &str, right: &str, file: &str, line: u32, col: u32) {
    // First, to improve sanity, remove trailing spaces from all lines
    let left = clean_trailing_spaces(left);
    let right = clean_trailing_spaces(right);

    if left != right {
        eprintln!("Assertion failed: Left != Right at {file}:{line}:{col}");
        eprintln!("Left:");
        eprintln!("{left}");
        eprintln!("Right:");
        eprintln!("{right}");

        let index = if let Some((index, (left_char, right_char))) = left
            .chars()
            .zip(right.chars())
            .enumerate()
            .find(|(_idx, (left_char, right_char))| left_char != right_char)
        {
            eprintln!("First difference at index = {index}, left_char {left_char:?} != right_char {right_char:?}\n");
            index
        } else {
            eprintln!("One string ends early");
            min(left.len(), right.len())
        };

        let window: usize = 15;
        eprintln!(
            "Left neighborhood:  {:?}",
            &left[index.saturating_sub(window)..min(index + window, left.len())]
        );
        eprintln!(
            "Right neighborhood: {:?}",
            &right[index.saturating_sub(window)..min(index + window, right.len())]
        );
        panic!("assertion failed");
    }
}

// Whenever ' ' occurs immediately before '\n', remove it
// Keep doing that until the string stops changing
fn clean_trailing_spaces(arg: &str) -> String {
    let mut result = arg.to_owned();

    loop {
        let next = result.replace(" \n", "\n");
        if next.len() == result.len() {
            return result;
        }
        result = next;
    }
}

#[macro_export]
macro_rules! assert_error_contains_text {
    ($left:expr, $right:expr) => {
        assert_error_contains_text(&$left, &$right, file!(), line!(), column!())
    };
    ($left:expr, $yes:expr, not $no:expr) => {
        assert_error_contains_text_and_not_other_text(
            &$left,
            &$yes,
            &$no,
            file!(),
            line!(),
            column!(),
        )
    };
}

pub fn assert_error_contains_text<T: core::fmt::Debug>(
    left: &Result<T, Error>,
    right: &[&str],
    file: &str,
    line: u32,
    col: u32,
) {
    match left {
        Ok(t) => {
            panic!("Assertion failed: expected error at {file}:{line}:{col}, found {left:#?}");
        }
        Err(e) => {
            let err_text = e.to_string();
            for substr in right {
                if !err_text.contains(substr) {
                    eprintln!("Assertion failed: error does not contain expected text at {file}:{line}:{col}");
                    eprintln!("Error text:");
                    eprintln!("{err_text}");
                    eprintln!("Expected substring:");
                    eprintln!("{substr}");
                    panic!("assertion failed");
                }
            }
        }
    }
}

pub fn assert_error_contains_text_and_not_other_text<T: core::fmt::Debug>(
    left: &Result<T, Error>,
    yes: &[&str],
    no: &[&str],
    file: &str,
    line: u32,
    col: u32,
) {
    match left {
        Ok(t) => {
            panic!("Assertion failed: expected error at {file}:{line}:{col}, found {left:#?}");
        }
        Err(e) => {
            let err_text = e.to_string();
            for substr in yes {
                if !err_text.contains(substr) {
                    eprintln!("Assertion failed: error does not contain expected text at {file}:{line}:{col}");
                    eprintln!("Error text:");
                    eprintln!("{err_text}");
                    eprintln!("Expected substring:");
                    eprintln!("{substr}");
                    panic!("assertion failed");
                }
            }
            for substr in no {
                if err_text.contains(substr) {
                    eprintln!("Assertion failed: error contains bad text at {file}:{line}:{col}");
                    eprintln!("Error text:");
                    eprintln!("{err_text}");
                    eprintln!("Bad substring:");
                    eprintln!("{substr}");
                    panic!("assertion failed");
                }
            }
        }
    }
}
