pub fn str_to_bool(src: &str) -> bool {
    !matches!(
        src.to_lowercase().as_str(),
        "" | "0" | "n" | "no" | "f" | "false" | "off"
    )
}
