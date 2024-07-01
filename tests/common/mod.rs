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

#[allow(unused)]
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
#[allow(unused)]
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
