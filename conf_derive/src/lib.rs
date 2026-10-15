use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, Error, Fields};
use syn::{DeriveInput, parse_macro_input};

mod proc_macro_options;
use proc_macro_options::GenConfStruct;

mod subcommand_proc_macro_options;
use subcommand_proc_macro_options::GenSubcommandsEnum;

pub(crate) mod util;

/// Derive a `Conf` implementation for an item with `#[conf(...)]` attributes
#[proc_macro_derive(Conf, attributes(conf, arg))]
pub fn conf(input: TokenStream1) -> TokenStream1 {
    let input: DeriveInput = parse_macro_input!(input);
    derive_conf(&input)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

fn derive_conf(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let ident = &input.ident;

    // Generic structs aren't supported yet (see #48), so give a clear error up front rather
    // than a confusing one from inside the expansion. The generated code already re-declares
    // the struct's generics on its nested helpers, but generics still need tests, docs, and
    // some design decisions before they can be enabled.
    if !input.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &input.generics,
            "#[derive(Conf)] does not support generic structs yet (lifetime, type, or const parameters)",
        ));
    }

    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let gener =
                GenConfStruct::new(ident, &input.vis, &input.generics, &input.attrs, fields)?;
            let conf_impl = gener.gen_conf_impl(&input.generics)?;
            let maybe_serde = gener.maybe_gen_conf_serde_impl(&input.generics)?;
            let maybe_test = gener.maybe_gen_test_fn(&input.generics)?;

            Ok(quote! {
                #conf_impl

                #maybe_serde

                #maybe_test
            })
        }

        _ => Err(Error::new(
            ident.span(),
            "#[derive(Conf)] is only supported on structs with named fields",
        )),
    }
}

/// Derive a `Subcommands` implementation for an item with `#[conf(...)]` attributes
#[proc_macro_derive(Subcommands, attributes(conf, arg))]
pub fn subcommands(input: TokenStream1) -> TokenStream1 {
    let input: DeriveInput = parse_macro_input!(input);
    derive_subcommands(&input)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

fn derive_subcommands(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let ident = &input.ident;

    match &input.data {
        Data::Enum(DataEnum { variants, .. }) => {
            let gener = GenSubcommandsEnum::new(ident, &input.attrs, variants.into_iter())?;
            gener.gen_all(&input.generics)
        }

        _ => Err(Error::new(
            ident.span(),
            "#[derive(Subcommands)] is only supported on enums",
        )),
    }
}
