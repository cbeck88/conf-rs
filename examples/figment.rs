#[cfg(feature = "serde")]
#[path = "serde/figment.rs"]
mod serde_figment;

#[cfg(feature = "serde")]
fn main() {
    serde_figment::main()
}

#[cfg(not(feature = "serde"))]
fn main() {
    panic!("needs serde feature")
}
