use super::StructItem;
use crate::util::type_is_bool;

use proc_macro2::TokenStream;
use syn::{punctuated::Punctuated, Field, Ident, Meta, Token, Type};

mod flag_item;
mod flatten_item;
mod parameter_item;
mod repeat_item;
mod subcommands_item;

use flag_item::FlagItem;
use flatten_item::FlattenItem;
use parameter_item::ParameterItem;
use repeat_item::RepeatItem;
use subcommands_item::SubcommandsItem;

/// #[conf(...)] options listed in a field of a struct which has `#[derive(Conf)]`
pub enum FieldItem {
    Flag(FlagItem),
    Parameter(ParameterItem),
    Repeat(RepeatItem),
    Flatten(FlattenItem),
    Subcommands(SubcommandsItem),
}

impl FieldItem {
    pub fn new(field: &Field, struct_item: &StructItem) -> Result<Self, syn::Error> {
        // First, inspect the first field attribute.
        // If the first attribute is 'flag', 'parameter', 'repeat', or 'flatten', then that's how
        // we're going to handle it.
        for attr in &field.attrs {
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
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
                    } else if path.is_ident("subcommands") {
                        return Ok(Self::Subcommands(SubcommandsItem::new(field, struct_item)?));
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

    /// True if this field represents a single program option
    pub fn is_single_option(&self) -> bool {
        matches!(
            self,
            Self::Flag(..) | Self::Parameter(..) | Self::Repeat(..)
        )
    }

    /// Get the field name
    pub fn get_field_name(&self) -> &Ident {
        match self {
            Self::Flag(item) => item.get_field_name(),
            Self::Parameter(item) => item.get_field_name(),
            Self::Repeat(item) => item.get_field_name(),
            Self::Flatten(item) => item.get_field_name(),
            Self::Subcommands(item) => item.get_field_name(),
        }
    }

    /// Get the field type
    pub fn get_field_type(&self) -> Type {
        match self {
            Self::Flag(item) => item.get_field_type(),
            Self::Parameter(item) => item.get_field_type(),
            Self::Repeat(item) => item.get_field_type(),
            Self::Flatten(item) => item.get_field_type(),
            Self::Subcommands(item) => item.get_field_type(),
        }
    }

    /// Generate code that constructs (one or more) ProgramOption as needed and pushes them onto
    /// program_options_ident
    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        match self {
            Self::Flag(item) => item.gen_push_program_options(program_options_ident),
            Self::Parameter(item) => item.gen_push_program_options(program_options_ident),
            Self::Repeat(item) => item.gen_push_program_options(program_options_ident),
            Self::Flatten(item) => item.gen_push_program_options(program_options_ident),
            Self::Subcommands(item) => item.gen_push_program_options(program_options_ident),
        }
    }

    /// Generate code that constructs (one or more) subcommands as needed and pushes them onto
    /// subcommands_ident
    pub fn gen_push_subcommands(
        &self,
        subcommands_ident: &Ident,
        parsed_env: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        match self {
            Self::Flag(item) => item.gen_push_subcommands(subcommands_ident, parsed_env),
            Self::Parameter(item) => item.gen_push_subcommands(subcommands_ident, parsed_env),
            Self::Repeat(item) => item.gen_push_subcommands(subcommands_ident, parsed_env),
            Self::Flatten(item) => item.gen_push_subcommands(subcommands_ident, parsed_env),
            Self::Subcommands(item) => item.gen_push_subcommands(subcommands_ident, parsed_env),
        }
    }

    /// Generate code for a struct initializer for this field
    ///
    /// Returns:
    /// * a TokenStream for initializer expression, which can use `?` to return errors,
    /// * a bool which is true if the error type is `Vec<InnerError>` and false if it is
    ///   `InnerError`
    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        match self {
            Self::Flag(item) => item.gen_initializer(conf_context_ident),
            Self::Parameter(item) => item.gen_initializer(conf_context_ident),
            Self::Repeat(item) => item.gen_initializer(conf_context_ident),
            Self::Flatten(item) => item.gen_initializer(conf_context_ident),
            Self::Subcommands(item) => item.gen_initializer(conf_context_ident),
        }
    }
}
