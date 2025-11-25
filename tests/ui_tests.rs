/// Compile-fail tests using trybuild
///
/// These tests verify that certain invalid configurations are caught at compile time
/// by the proc macro with appropriate error messages.

#[test]
fn ui_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
