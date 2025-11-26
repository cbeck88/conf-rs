#![cfg(feature = "serde")]

/// Compile-fail tests for serde-specific features using trybuild
///
/// These tests verify that invalid serde attribute combinations are caught at compile time
/// by the proc macro with appropriate error messages.

#[test]
fn ui_tests_serde() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui_serde/*.rs");
}
