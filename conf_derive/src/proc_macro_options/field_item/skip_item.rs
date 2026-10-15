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
    pub fn new(field: &Field) -> Result<Self, Error> {
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

    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
        generics: &NestedGenerics,
    ) -> Result<(TokenStream, bool), Error> {
        let (g_decl, g_turbofish, g_where) = generics.fn_parts();
        let field_type = &self.field_type;
        let value = match &self.value {
            Some(expr) => quote! { #expr },
            None => quote! { ::core::default::Default::default() },
        };
        // The value goes in its own nested fn, like a parameter's default(...), so that the
        // user's expression can't see our local variables.
        Ok((
            quote! {
                fn __skip_value__ #g_decl () -> #field_type #g_where {
                    #value
                }
                let _ = #conf_context_ident;
                Ok(__skip_value__ #g_turbofish ())
            },
            false,
        ))
    }
}
