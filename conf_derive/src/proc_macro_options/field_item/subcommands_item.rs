use super::StructItem;
use crate::util::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Error, Field, Ident, Type};

pub struct SubcommandsItem {
    struct_name: Ident,
    field_name: Ident,
    field_type: Type,
    is_optional_type: Option<Type>,
    doc_string: Option<String>,
}

impl SubcommandsItem {
    pub fn new(field: &Field, struct_item: &StructItem) -> Result<Self, Error> {
        let struct_name = struct_item.struct_ident.clone();
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "missing identifier"))?;
        let field_type = field.ty.clone();
        let is_optional_type = type_is_option(&field.ty)?;

        let mut result = Self {
            struct_name,
            field_name,
            field_type,
            is_optional_type,
            doc_string: None,
        };

        for attr in &field.attrs {
            maybe_append_doc_string(&mut result.doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("subcommands") {
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized conf subcommands option"))
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

    // Subcommands fields don't add any program options to the conf structure.
    pub fn gen_push_program_options(
        &self,
        _program_options_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        Ok(quote! {})
    }

    // Subcommands fields add subcommand parsers to the conf structure.
    pub fn gen_push_subcommands(
        &self,
        parsers_ident: &Ident,
        parsed_env_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        let inner_type: &Type = self.is_optional_type.as_ref().unwrap_or(&self.field_type);
        let panic_message = format!(
            "Not supported to have multiple subcommands fields on the same struct: at field '{}'",
            self.field_name
        );

        // TODO: In theory we could support having multiple subcommands fields on the same struct,
        // but it's not clear that it's useful.
        Ok(quote! {
            if !#parsers_ident.is_empty() {
                panic!(#panic_message);
            }
            #parsers_ident.extend(<#inner_type as ::conf::Subcommands>::get_parsers(#parsed_env_ident)?);
        })
    }

    // Body of a function taking a &ConfContext returning Result<#field_type,
    // Vec<::conf::InnerError>>
    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        let struct_name = self.struct_name.to_string();
        let field_name = self.field_name.to_string();
        let field_type = &self.field_type;

        let result = if let Some(inner_type) = self.is_optional_type.as_ref() {
            quote! {
                if let Some((name, conf_context)) = #conf_context_ident.for_subcommand() {
                    Ok(Some(<#inner_type as ::conf::Subcommands>::from_conf_context(name, conf_context)?))
                } else {
                    Ok(None)
                }
            }
        } else {
            quote! {
                let Some((name, conf_context)) = #conf_context_ident.for_subcommand() else {
                    return Err(vec![ ::conf::InnerError::missing_required_subcommand( #struct_name, #field_name, <#field_type as ::conf::Subcommands>::get_subcommand_names() ) ]);
                };
                Ok(<#field_type as ::conf::Subcommands>::from_conf_context(name, conf_context)?)
            }
        };

        Ok((result, true))
    }
}
