//! These are helper structures which:
//! * Parse the `#[conf(...)]` attributes that appear on different types of items
//! * Store the results and make them easily available
//! * Assist with subsequent codegen
//!
//! This contains such helpers for the derive(Subcommand) macro.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, Error, Generics, Ident, Variant, parse_quote};

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
    generics: Generics,
}

impl GenSubcommandsEnum {
    /// Parse syn data for an enum with `#[derive(Subcommands)]` on it
    pub fn new<'a>(
        ident: &Ident,
        generics: &Generics,
        attrs: &[Attribute],
        variants: impl Iterator<Item = &'a Variant>,
    ) -> Result<Self, Error> {
        let variants = variants
            .map(|var| VariantItem::new(var, ident, generics))
            .collect::<Result<Vec<_>, _>>()?;

        // Named-field variants become generated structs with derive(Conf), which get all of the
        // enum's generics, and derive(Conf) doesn't support lifetime params.
        if let Some(lifetime) = generics.lifetimes().next() {
            if variants.iter().any(|var| var.get_named_fields().is_some()) {
                return Err(Error::new_spanned(
                    lifetime,
                    "#[derive(Subcommands)] doesn't support lifetime parameters on an enum with named-field variants",
                ));
            }
        }

        Ok(Self {
            enum_item: EnumItem::new(ident, attrs)?,
            variants,
            generics: generics.clone(),
        })
    }

    /// Generate hidden structs for variants with named fields.
    /// Uses mangled names to avoid shadowing types referenced in field types.
    ///
    /// If the enum is generic, each struct gets all of the enum's generics and where clause, plus
    /// a skipped PhantomData field, since its fields might not use every type param. (Working out
    /// which generics and where predicates each variant actually needs isn't worth it.)
    fn gen_named_field_structs(&self) -> TokenStream {
        let (impl_generics, _, where_clause) = self.generics.split_for_impl();
        let type_params = self.generics.type_params().map(|tp| &tp.ident);
        let phantom_field: Option<syn::Field> = (!self.generics.params.is_empty()).then(|| {
            parse_quote! {
                #[conf(skip)]
                __variant_phantom__: ::core::marker::PhantomData<fn() -> (#(#type_params,)*)>
            }
        });

        let structs: Vec<TokenStream> = self
            .variants
            .iter()
            .filter_map(|var| {
                var.get_named_fields().map(|fields| {
                    let mut fields = fields.clone();
                    fields.named.extend(phantom_field.clone());
                    let struct_name = var.get_generated_struct_name();
                    let display_name = var.get_display_name();
                    // Add #[conf(serde)] if enum has serde AND this variant isn't skipped
                    let serde_attr = if self.enum_item.serde && !var.get_serde_skip() {
                        quote! { #[conf(serde)] }
                    } else {
                        quote! {}
                    };
                    // Add passthrough attributes (one_of_fields, validation_predicate, etc.)
                    let passthrough_attrs = var.gen_passthrough_conf_attrs();
                    quote! {
                        #[derive(::conf::Conf)]
                        #[allow(non_camel_case_types)]
                        #[conf(display_name = #display_name)]
                        #passthrough_attrs
                        #serde_attr
                        struct #struct_name #impl_generics #where_clause #fields
                    }
                })
            })
            .collect();

        quote! { #(#structs)* }
    }

    /// Generate all code for the Subcommands derive macro.
    /// Everything is wrapped in `const _: () = { ... }` to provide an anonymous namespace.
    pub fn gen_all(&self, generics: &Generics) -> Result<TokenStream, syn::Error> {
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let ident = self.enum_item.get_ident();

        let named_field_structs = self.gen_named_field_structs();

        let subcommands_fns = vec![
            self.get_parsers_impl()?,
            self.get_subcommand_names_impl()?,
            self.from_conf_context_impl()?,
            self.debug_asserts_impl()?,
        ];

        let subcommands_impl = quote! {
          #[automatically_derived]
          #[allow(
            unused_qualifications,
          )]
          impl #impl_generics ::conf::Subcommands for #ident #ty_generics #where_clause {
            #(#subcommands_fns)*
          }
        };

        let serde_impl = self.gen_subcommands_serde_impl(generics)?;

        Ok(quote! {
            const _: () = {
                #named_field_structs

                #subcommands_impl

                #serde_impl
            };
        })
    }

    /// Generate Subcommands::get_parsers implementation
    fn get_parsers_impl(&self) -> Result<TokenStream, syn::Error> {
        let parsers_ident = Ident::new("__parsers__", Span::call_site());
        let parsed_env_ident = Ident::new("__parsed_env__", Span::call_site());
        let variants_push_parsers: Vec<TokenStream> = self
            .variants
            .iter()
            .map(|var| var.gen_push_parser(&parsers_ident, &parsed_env_ident))
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
        let all_command_names: Vec<_> = self
            .variants
            .iter()
            .flat_map(|var| var.get_all_command_names())
            .collect();

        Ok(quote! {
          fn get_subcommand_names() -> &'static [&'static str] {
            &[ #(#all_command_names,)* ]
          }
        })
    }

    /// Generate Subcommands::from_conf_context implementation
    #[allow(clippy::wrong_self_convention)]
    fn from_conf_context_impl(&self) -> Result<TokenStream, syn::Error> {
        let conf_context_ident = Ident::new("conf_context__", Span::call_site());
        let variant_match_arms: Vec<TokenStream> = self
            .variants
            .iter()
            .map(|var| var.gen_from_conf_context_match_arm(&conf_context_ident))
            .collect::<Result<Vec<_>, syn::Error>>()?;

        Ok(quote! {
          fn from_conf_context(
            command_name: String,
            #conf_context_ident: ::conf::ConfContext<'_>
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

    /// Generate Subcommands::debug_asserts implementation
    fn debug_asserts_impl(&self) -> Result<TokenStream, syn::Error> {
        let assertions: Vec<TokenStream> = self
            .variants
            .iter()
            .map(|variant| variant.gen_debug_asserts())
            .collect::<Result<Vec<_>, syn::Error>>()?;

        Ok(quote! {
            fn debug_asserts() {
                #(#assertions)*
            }
        })
    }

    /// Generate a SubcommandsSerde impl for this enum, if serde is enabled.
    /// Returns empty TokenStream if serde is not enabled.
    fn gen_subcommands_serde_impl(&self, generics: &Generics) -> Result<TokenStream, syn::Error> {
        if !self.enum_item.serde {
            return Ok(quote! {});
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let ident = self.enum_item.get_ident();

        let subcommands_serde_items =
            vec![self.gen_serde_names()?, self.gen_from_conf_serde_context()?];

        Ok(quote! {
          #[automatically_derived]
          #[allow(
            unused_qualifications,
          )]
          impl #impl_generics ::conf::SubcommandsSerde for #ident #ty_generics #where_clause {
            #(#subcommands_serde_items)*
          }
        })
    }

    fn gen_serde_names(&self) -> Result<TokenStream, syn::Error> {
        let tuples: Vec<TokenStream> = self
            .variants
            .iter()
            .filter(|var| !var.get_serde_skip())
            .flat_map(|var| {
                let command_name = var.get_command_name();
                let serde_name = var.get_serde_name();
                let serde_aliases = var.get_serde_aliases();

                // Create a tuple for the primary serde name
                let primary_tuple = quote! {
                    (#command_name, #serde_name)
                };

                // Create tuples for all aliases
                let alias_tuples: Vec<TokenStream> = serde_aliases
                    .iter()
                    .map(|alias| {
                        quote! {
                            (#command_name, #alias)
                        }
                    })
                    .collect();

                std::iter::once(primary_tuple).chain(alias_tuples)
            })
            .collect();
        Ok(quote! {
            const SERDE_NAMES: &'static[(&'static str, &'static str)] = &[ #( #tuples ),* ];
        })
    }

    fn gen_from_conf_serde_context(&self) -> Result<TokenStream, syn::Error> {
        let conf_context_ident = Ident::new("ctxt", Span::call_site());
        let next_value_producer_ident = Ident::new("__next_value_producer__", Span::call_site());

        let variant_match_arms: Vec<TokenStream> = self
            .variants
            .iter()
            .filter(|var| !var.get_serde_skip())
            .map(|var| {
                var.gen_from_conf_serde_context_match_arm(
                    &conf_context_ident,
                    &next_value_producer_ident,
                )
            })
            .collect::<Result<Vec<_>, syn::Error>>()?;

        Ok(quote! {
            fn from_conf_serde_context<'de, NVP>(
               command_name: &str,
               #conf_context_ident: ::conf::ConfSerdeContext,
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
