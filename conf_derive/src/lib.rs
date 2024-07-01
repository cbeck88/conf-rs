use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};
use syn::{Data, DataStruct, Error, Fields, Generics, Ident};

mod proc_macro_options;
use proc_macro_options::{collect_args_fields, FieldItem, StructItem};

pub(crate) mod util;

/// Derive a `Conf` implementation for an item with `#[conf(...)]` attributes
#[proc_macro_derive(Conf, attributes(conf))]
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
            let struct_item = StructItem::new(&input.attrs)?;
            let fields = collect_args_fields(&struct_item, fields)?;
            gen_conf_impl_for_struct(&struct_item, ident, &input.generics, &fields)
        }

        _ => Err(Error::new(
            ident.span(),
            "#[derive(Conf)] is only supported on structs with named fields",
        )),
    }
}

fn gen_conf_impl_for_struct(
    struct_item: &StructItem,
    item_name: &Ident,
    generics: &Generics,
    fields: &[FieldItem],
) -> Result<TokenStream, syn::Error> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // To implement Conf::get_program_options, we need to get parser_config
    // for this struct, (top-level config essentially), and then we also need to
    // get all the program options for our constituents. To do this, we create
    // an ident for the list of program options, which is going to be Vec<ProgramOption>.
    // Then we pass that ident to every constitutent field, and aggregate all their code gen.
    let program_options_ident = Ident::new("program_options", Span::call_site());
    let parser_config = struct_item.gen_parser_config()?;
    let fields_push_program_options: Vec<TokenStream> = fields
        .iter()
        .map(|field| field.gen_push_program_options(&program_options_ident))
        .collect::<Result<Vec<_>, syn::Error>>()?;

    // To implement Conf::from_conf_context, we need to take a conf context,
    // and then return Ok(Self { ... }). For each constituent field, we need it
    // to generate code to initialize itself properly. We pass the ConfContext ident
    // to each consittuent field, and then aggregate all their code gen.
    // Their code-gen is allowed to use `?` or `return Err(...)` to early return.
    let conf_context_ident = Ident::new("conf_context", Span::call_site());
    let initializations: Vec<TokenStream> = fields
        .iter()
        .map(|field| field.gen_initializer(&conf_context_ident))
        .collect::<Result<Vec<_>, syn::Error>>()?;

    // To implement #[conf(env_prefix="ACME_")] on a struct (rather than on a flattened field),
    // the code gen associated to the struct needs to be able to add its own prefixing during get_program_options
    // and during from_conf_context.
    // To do this, we allow the struct_item to "post-process" the Vec<ProgramOption>, (to add a prefix to them all)
    // and to "pre-process" the ConfContext (to add a matching prefix to that before it is used)
    let struct_post_process_program_options =
        struct_item.gen_post_process_program_options(&program_options_ident)?;
    let struct_pre_process_conf_context =
        struct_item.gen_pre_process_conf_context(&conf_context_ident)?;

    Ok(quote! {
        #[automatically_derived]
        #[allow(
            unused_qualifications,
        )]
        impl #impl_generics conf::Conf for #item_name #ty_generics #where_clause {
            fn get_program_options() -> Result<(conf::ParserConfig, Vec<conf::ProgramOption>), conf::Error> {
                let parser_config = #parser_config;
                let mut #program_options_ident = vec![];

                #(#fields_push_program_options)*

                #struct_post_process_program_options

                Ok((parser_config, #program_options_ident))
            }

            fn from_conf_context(#conf_context_ident: conf::ConfContext<'_>) -> Result<Self, conf::Error> {
                #struct_pre_process_conf_context

                Ok(Self {
                    #(#initializations)*
                })
            }
        }
    })
}
