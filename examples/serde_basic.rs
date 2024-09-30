#[cfg(feature = "serde")]
#[path = "serde/basic.rs"]
mod basic;

#[cfg(feature = "serde")]
fn main() {
    basic::main()
}

#[cfg(not(feature = "serde"))]
fn main() {
    panic!("needs serde feature")
}
