//! These are helper structures which:
//! * Parse the `#[conf(...)]` attributes that appear on different types of items
//! * Store the results and make them easily available
//! * Assist with subsequent codegen

use syn::FieldsNamed;

mod field_item;
mod struct_item;

pub use field_item::FieldItem;
pub use struct_item::StructItem;

/// Helper for parsing field items out of a syn struct
pub fn collect_args_fields(
    struct_item: &StructItem,
    fields: &FieldsNamed,
) -> Result<Vec<FieldItem>, syn::Error> {
    fields
        .named
        .iter()
        .map(|field| FieldItem::new(field, struct_item))
        .collect()
}
