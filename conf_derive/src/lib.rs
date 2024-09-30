use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};
use syn::{Data, DataEnum, DataStruct, Error, Fields};

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

    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let gen = GenConfStruct::new(ident, &input.attrs, fields)?;
            let conf_impl = gen.gen_conf_impl(&input.generics)?;
            let maybe_serde = gen.maybe_gen_conf_serde_impl(&input.generics)?;

            Ok(quote! {
                #conf_impl

                #maybe_serde
            })
        }

        _ => Err(Error::new(
            ident.span(),
            "#[derive(Conf)] is only supported on structs with named fields",
        )),
    }
}

/// Derive a `Subcommands` implementation for an item with `#[conf(...)]` attributes
#[proc_macro_derive(Subcommands, attributes(conf))]
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
            let gen = GenSubcommandsEnum::new(ident, &input.attrs, variants.into_iter())?;
            let subcommands_impl = gen.gen_subcommands_impl(&input.generics)?;
            let maybe_serde = gen.maybe_gen_subcommands_serde_impl(&input.generics)?;

            Ok(quote! {
                #subcommands_impl

                #maybe_serde
            })
        }

        _ => Err(Error::new(
            ident.span(),
            "#[derive(Subcommands)] is only supported on enums",
        )),
    }
}
