//! These are helper structures which:
//! * Parse the `#[conf(...)]` attributes that appear on different types of items
//! * Store the results and make them easily available
//! * Assist with subsequent codegen
//!
//! This contains such helpers for the derive(Subcommand) macro.

use syn::{Ident, Variant};

mod variant_item;
pub use variant_item::VariantItem;

/// Helper for parsing variant items out of a syn enum, tagged as a group of subcommands
pub fn collect_enum_variants<'a>(
    enum_ident: &Ident,
    variants: impl Iterator<Item = &'a Variant>,
) -> Result<Vec<VariantItem>, syn::Error> {
    variants
        .map(|var| VariantItem::new(var, enum_ident))
        .collect()
}
