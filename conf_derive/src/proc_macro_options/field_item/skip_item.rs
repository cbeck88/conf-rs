use super::StructItem;
use crate::util::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Expr, Field, Ident, Type, spanned::Spanned};

/// Proc macro annotations parsed from a field marked `#[conf(skip)]`
///
/// The field isn't a program option at all. It doesn't read from args, env, or serde, it doesn't
/// appear in help, and it's initialized with `Default::default()`, or with the given expression.
pub struct SkipItem {
    field_name: Ident,
    field_type: Type,
    value: Option<Expr>,
}

impl SkipItem {
    pub fn new(field: &Field, _struct_item: &StructItem) -> Result<Self, Error> {
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "missing identifier"))?;

        let mut result = Self {
            field_name,
            field_type: field.ty.clone(),
            value: None,
        };

        let mut seen_skip = false;
        for attr in &field.attrs {
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("skip") {
                        if seen_skip {
                            return Err(meta.error("`skip` can only be set once"));
                        }
                        seen_skip = true;
                        result.value = parse_optional_value::<Expr>(meta)?;
                        Ok(())
                    } else {
                        Err(meta.error(
                            "`skip` can't be combined with other conf attributes, since a skipped field isn't parsed from anywhere",
                        ))
                    }
                })?;
            }
        }

        Ok(result)
    }

    pub fn get_field_name(&self) -> &Ident {
        &self.field_name
    }

    pub fn get_field_type(&self) -> Type {
        self.field_type.clone()
    }

    pub fn gen_push_subcommands(
        &self,
        _subcommands_ident: &Ident,
        _parsed_env: &Ident,
    ) -> Result<TokenStream, Error> {
        Ok(quote! {})
    }

    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), Error> {
        let value = match &self.value {
            Some(expr) => quote! { #expr },
            None => quote! { ::core::default::Default::default() },
        };
        Ok((
            quote! {
                let _ = #conf_context_ident;
                Ok(#value)
            },
            false,
        ))
    }

    /// Skipped fields don't need debug assertions
    pub fn gen_debug_asserts(&self, _struct_ident: &Ident) -> Result<TokenStream, Error> {
        Ok(quote! {})
    }
}
