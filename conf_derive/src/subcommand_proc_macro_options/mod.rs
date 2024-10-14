//! These are helper structures which:
//! * Parse the `#[conf(...)]` attributes that appear on different types of items
//! * Store the results and make them easily available
//! * Assist with subsequent codegen
//!
//! This contains such helpers for the derive(Subcommand) macro.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, Error, Generics, Ident, Variant};

mod enum_item;
use enum_item::EnumItem;

mod variant_item;
use variant_item::VariantItem;

/// Helper which generates individual functions related to `#[derive(Subcommands)]`
/// on an enum.
///
/// Calling "new" parses all the proc macro attributes for enum and fields.
/// Calling individual functions returns code gen.
pub struct GenSubcommandsEnum {
    enum_item: EnumItem,
    variants: Vec<VariantItem>,
}

impl GenSubcommandsEnum {
    /// Parse syn data for an enum with `#[derive(Subcommands)]` on it
    pub fn new<'a>(
        ident: &Ident,
        attrs: &[Attribute],
        variants: impl Iterator<Item = &'a Variant>,
    ) -> Result<Self, Error> {
        Ok(Self {
            enum_item: EnumItem::new(ident, attrs)?,
            variants: variants
                .map(|var| VariantItem::new(var, ident))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    /// Generate a Subcommands impl for this enum
    pub fn gen_subcommands_impl(&self, generics: &Generics) -> Result<TokenStream, syn::Error> {
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let ident = self.enum_item.get_ident();

        let subcommands_fns = vec![
            self.get_parsers_impl()?,
            self.get_subcommand_names_impl()?,
            self.from_conf_context_impl()?,
        ];

        Ok(quote! {
          #[automatically_derived]
          #[allow(
            unused_qualifications,
          )]
          impl #impl_generics ::conf::Subcommands for #ident #ty_generics #where_clause {
            #(#subcommands_fns)*
          }
        })
    }

    /// Generate Subcommands::get_parsers implementation
    fn get_parsers_impl(&self) -> Result<TokenStream, syn::Error> {
        let parsers_ident = Ident::new("__parsers__", Span::call_site());
        let parsed_env_ident = Ident::new("__parsed_env__", Span::call_site());
        let variants_push_parsers: Vec<TokenStream> = self
            .variants
            .iter()
            .map(|var| var.gen_push_parsers(&parsers_ident, &parsed_env_ident))
            .collect::<Result<Vec<_>, syn::Error>>()?;

        Ok(quote! {
          fn get_parsers(#parsed_env_ident: &::conf::ParsedEnv) -> Result<Vec<::conf::Parser>, ::conf::Error> {
            let mut #parsers_ident = vec![];

            #(#variants_push_parsers)*

            Ok(#parsers_ident)
          }
        })
    }

    /// Generate Subcommands::get_subcommand_names implementation
    fn get_subcommand_names_impl(&self) -> Result<TokenStream, syn::Error> {
        let command_names: Vec<_> = self
            .variants
            .iter()
            .map(|var| var.get_command_name())
            .collect();

        Ok(quote! {
          fn get_subcommand_names() -> &'static [&'static str] {
            &[ #(#command_names,)* ]
          }
        })
    }

    /// Generate Subcommands::from_conf_context implementation
    #[allow(clippy::wrong_self_convention)]
    fn from_conf_context_impl(&self) -> Result<TokenStream, syn::Error> {
        let variant_match_arms: Vec<TokenStream> = self.variants
            .iter()
            .map(|var| {
                let name = var.get_name();
                let command_name = var.get_command_name();
                let ty = var.get_type();
                quote! {
                    #command_name => Ok(Self::#name(<#ty as Conf>::from_conf_context(conf_context)?))
                }
            })
            .collect();

        Ok(quote! {
          fn from_conf_context(
            command_name: String,
            conf_context: ::conf::ConfContext<'_>
          ) -> Result<Self, Vec<::conf::InnerError>> {
            match command_name.as_str() {
              #(#variant_match_arms,)*
              _ => {
                panic!(
                  "Unknown command name '{command_name}'. This is an internal error. Expected '{:?}'",
                  <Self as ::conf::Subcommands>::get_subcommand_names()
                )
              }
            }
          }
        })
    }

    /// Generate a SubcommandsSerde impl for this enum, if requested
    pub fn maybe_gen_subcommands_serde_impl(
        &self,
        generics: &Generics,
    ) -> Result<Option<TokenStream>, syn::Error> {
        if !self.enum_item.serde {
            return Ok(None);
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let ident = self.enum_item.get_ident();

        let subcommands_serde_items =
            vec![self.gen_serde_names()?, self.gen_from_conf_serde_context()?];

        Ok(Some(quote! {
          #[automatically_derived]
          #[allow(
            unused_qualifications,
          )]
          impl #impl_generics ::conf::SubcommandsSerde for #ident #ty_generics #where_clause {
            #(#subcommands_serde_items)*
          }
        }))
    }

    fn gen_serde_names(&self) -> Result<TokenStream, syn::Error> {
        let tuples: Vec<TokenStream> = self
            .variants
            .iter()
            .filter(|var| !var.get_serde_skip())
            .map(|var| {
                let command_name = var.get_command_name();
                let serde_name = var.get_serde_name();
                quote! {
                    (#command_name, #serde_name)
                }
            })
            .collect();
        Ok(quote! {
            const SERDE_NAMES: &'static[(&'static str, &'static str)] = &[ #( #tuples ),* ];
        })
    }

    fn gen_from_conf_serde_context(&self) -> Result<TokenStream, syn::Error> {
        let next_value_producer_ident = Ident::new("__next_value_producer__", Span::call_site());

        let variant_match_arms: Vec<TokenStream> = self
            .variants
            .iter()
            .filter(|var| !var.get_serde_skip())
            .map(|var| {
                let name = var.get_name();
                let command_name = var.get_command_name();
                let serde_name = var.get_serde_name();
                let ty = var.get_type();
                quote! {
                    #command_name => {
                      let document_name = ctxt.document_name;
                      let seed = <#ty as ConfSerde>::Seed::from(ctxt);
                      Ok(Self::#name(#next_value_producer_ident.next_value_seed(seed).map_err(|err| {
                       vec![InnerError::serde(
                         document_name,
                         #serde_name,
                         err
                       )]
                      })??))
                    }
                }
            })
            .collect();

        Ok(quote! {
            fn from_conf_serde_context<'de, NVP>(
               command_name: &str,
               ctxt: ::conf::ConfSerdeContext,
               #next_value_producer_ident: NVP
            ) -> Result<Self, Vec<::conf::InnerError>>
               where NVP: ::conf::NextValueProducer<'de>
            {
                use ::conf::{ConfSerde, InnerError};
                match command_name {
                  #(#variant_match_arms,)*
                  _ => {
                    panic!(
                      "Unknown command name '{command_name}'. This is an internal error. Expected '{:?}'",
                      <Self as ::conf::SubcommandsSerde>::SERDE_NAMES
                    )
                  }
                }
            }
        })
    }
}
