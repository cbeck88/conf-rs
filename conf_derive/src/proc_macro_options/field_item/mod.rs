use super::StructItem;
use crate::util::type_is_bool;

use proc_macro2::TokenStream;
use syn::{punctuated::Punctuated, Field, Ident, Meta, Token};

mod flag_item;
mod flatten_item;
mod parameter_item;
mod repeat_item;

use flag_item::FlagItem;
use flatten_item::FlattenItem;
use parameter_item::ParameterItem;
use repeat_item::RepeatItem;

/// #[conf(...)] options listed in a field of a struct which has `#[derive(Conf)]`
pub enum FieldItem {
    Flag(FlagItem),
    Parameter(ParameterItem),
    Repeat(RepeatItem),
    Flatten(FlattenItem),
}

impl FieldItem {
    pub fn new(field: &Field, struct_item: &StructItem) -> Result<Self, syn::Error> {
        // First, inspect the first field attribute.
        // If the first attribute is 'flag', 'parameter', 'repeat', or 'flatten', then that's how we're going to handle it.
        for attr in &field.attrs {
            if attr.path().is_ident("conf") {
                let nested =
                    attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
                if let Some(meta) = nested.first() {
                    let path = meta.path();
                    if path.is_ident("flag") {
                        return Ok(Self::Flag(FlagItem::new(field, struct_item)?));
                    } else if path.is_ident("parameter") {
                        return Ok(Self::Parameter(ParameterItem::new(field, struct_item)?));
                    } else if path.is_ident("repeat") {
                        return Ok(Self::Repeat(RepeatItem::new(field, struct_item)?));
                    } else if path.is_ident("flatten") {
                        return Ok(Self::Flatten(FlattenItem::new(field, struct_item)?));
                    }
                }
            }
        }

        // We're still not sure, so inspect the type.
        // If it's bool, it's a flag. Otherwise it's a parameter.
        Ok(if type_is_bool(&field.ty) {
            Self::Flag(FlagItem::new(field, struct_item)?)
        } else {
            Self::Parameter(ParameterItem::new(field, struct_item)?)
        })
    }

    /// Generate code that constructs (one or more) ProgramOption as needed and pushes them onto program_options_ident
    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        match self {
            Self::Flag(item) => item.gen_push_program_options(program_options_ident),
            Self::Parameter(item) => item.gen_push_program_options(program_options_ident),
            Self::Repeat(item) => item.gen_push_program_options(program_options_ident),
            Self::Flatten(item) => item.gen_push_program_options(program_options_ident),
        }
    }

    /// Generate code for a struct initializer for this field
    pub fn gen_initializer(&self, conf_context_ident: &Ident) -> Result<TokenStream, syn::Error> {
        match self {
            Self::Flag(item) => item.gen_initializer(conf_context_ident),
            Self::Parameter(item) => item.gen_initializer(conf_context_ident),
            Self::Repeat(item) => item.gen_initializer(conf_context_ident),
            Self::Flatten(item) => item.gen_initializer(conf_context_ident),
        }
    }
}
