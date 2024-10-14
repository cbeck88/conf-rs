use crate::util::*;
use syn::{Attribute, Error, Ident};

/// #[conf(...)] options listed on an enum which has `#[derive(Subcommands)]`
///
/// Also assists with code generation related to these, such as for validations
pub struct EnumItem {
    pub enum_ident: Ident,
    pub serde: bool,
    pub doc_string: Option<String>,
}

impl EnumItem {
    /// Parse conf options out of attributes on an enum
    pub fn new(enum_ident: &Ident, attrs: &[Attribute]) -> Result<Self, Error> {
        let mut result = Self {
            enum_ident: enum_ident.clone(),
            serde: false,
            doc_string: None,
        };

        for attr in attrs {
            maybe_append_doc_string(&mut result.doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("serde") {
                        result.serde = true;
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized conf option"))
                    }
                })?;
            }
        }

        Ok(result)
    }

    /// Get the identifier of this enum
    pub fn get_ident(&self) -> &Ident {
        &self.enum_ident
    }
}
