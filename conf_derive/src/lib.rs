use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};
use syn::{Data, DataEnum, DataStruct, Error, Fields, Generics, Ident};

mod proc_macro_options;
use proc_macro_options::{collect_args_fields, FieldItem, StructItem};

mod subcommand_proc_macro_options;
use subcommand_proc_macro_options::{collect_enum_variants, VariantItem};

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
            let struct_item = StructItem::new(ident, &input.attrs)?;
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

    let get_parser_config_impl = gen_conf_get_parser_config_impl_for_struct(struct_item, fields)?;
    let get_program_options_impl =
        gen_conf_get_program_options_impl_for_struct(struct_item, fields)?;
    let get_subcommands_impl = gen_conf_get_subcommands_impl_for_struct(struct_item, fields)?;
    let from_conf_context_impl = gen_conf_from_conf_context_impl_for_struct(struct_item, fields)?;
    let get_name_impl = gen_conf_get_name_impl_for_struct(struct_item)?;

    Ok(quote! {
        #[automatically_derived]
        #[allow(
            unused_qualifications,
        )]
        impl #impl_generics conf::Conf for #item_name #ty_generics #where_clause {
            #get_parser_config_impl
            #get_program_options_impl
            #get_subcommands_impl
            #from_conf_context_impl
            #get_name_impl
        }
    })
}

fn gen_conf_get_name_impl_for_struct(struct_item: &StructItem) -> Result<TokenStream, syn::Error> {
    let struct_name = struct_item.get_ident().to_string();

    Ok(quote! {
        fn get_name() -> &'static str {
            #struct_name
        }
    })
}

fn gen_conf_get_parser_config_impl_for_struct(
    struct_item: &StructItem,
    _fields: &[FieldItem],
) -> Result<TokenStream, syn::Error> {
    // To implement Conf::get_parser_config, we need to get parser_config
    // for this struct, (top-level config essentially).
    let parser_config = struct_item.gen_parser_config()?;

    Ok(quote! {
        fn get_parser_config() -> Result<conf::ParserConfig, conf::Error> {
            let parser_config = #parser_config;

            Ok(parser_config)
        }
    })
}

fn gen_conf_get_program_options_impl_for_struct(
    struct_item: &StructItem,
    fields: &[FieldItem],
) -> Result<TokenStream, syn::Error> {
    // To implement Conf::get_program_options, we need to
    // get all the program options for our constituents. To do this, we create
    // an ident for the list of program options, which is going to be Vec<ProgramOption>.
    // Then we pass that ident to every constitutent field, and aggregate all their code gen.
    let program_options_ident = Ident::new("program_options", Span::call_site());
    let fields_push_program_options: Vec<TokenStream> = fields
        .iter()
        .map(|field| field.gen_push_program_options(&program_options_ident))
        .collect::<Result<Vec<_>, syn::Error>>()?;

    // To implement #[conf(env_prefix="ACME_")] on a struct (rather than on a flattened field),
    // the code gen associated to the struct needs to be able to add its own prefixing during
    // get_program_options and during from_conf_context.
    // To do this, we allow the struct_item to "post-process" the Vec<ProgramOption>, (to add a
    // prefix to them all) and to "pre-process" the ConfContext (to add a matching prefix to
    // that before it is used) Note: The preprocessing no longer does anything since we switched
    // to using id's like clap does.
    let struct_post_process_program_options =
        struct_item.gen_post_process_program_options(&program_options_ident)?;

    // Note: fields_push_program_options is allowed to early return with ? on an error
    Ok(quote! {
        fn get_program_options() -> Result<&'static [conf::ProgramOption], conf::Error> {
            static CACHED: ::std::sync::OnceLock<Vec<::conf::ProgramOption>> = ::std::sync::OnceLock::new();

            if CACHED.get().is_none() {
                let mut #program_options_ident = vec![];

                #(#fields_push_program_options)*

                #struct_post_process_program_options

                let _ = CACHED.set(#program_options_ident);
            }

            let cached = CACHED.get().unwrap();

            Ok(cached.as_ref())
        }
    })
}

fn gen_conf_get_subcommands_impl_for_struct(
    _struct_item: &StructItem,
    fields: &[FieldItem],
) -> Result<TokenStream, syn::Error> {
    let parsers_ident = Ident::new("__parsers__", Span::call_site());
    let parsed_env_ident = Ident::new("__parsed_env__", Span::call_site());
    let fields_push_subcommands: Vec<TokenStream> = fields
        .iter()
        .map(|field| field.gen_push_subcommands(&parsers_ident, &parsed_env_ident))
        .collect::<Result<Vec<_>, syn::Error>>()?;

    Ok(quote! {
        fn get_subcommands(#parsed_env_ident: &::conf::ParsedEnv) -> Result<Vec<conf::Parser>, conf::Error> {
            let mut #parsers_ident = vec![];

            #(#fields_push_subcommands)*

            Ok(#parsers_ident)
        }
    })
}

fn gen_conf_from_conf_context_impl_for_struct(
    struct_item: &StructItem,
    fields: &[FieldItem],
) -> Result<TokenStream, syn::Error> {
    // To implement Conf::from_conf_context, we need to take a conf context,
    // and then return Ok(Self { ... }). For each constituent field, we need it
    // to generate code to initialize itself properly. We pass the ConfContext ident
    // to each consittuent field, and then aggregate all their code gen.
    // Their code-gen is allowed to use `?` or `return Err(...)` to early return,
    // but we still need to aggregate all the errors. Sample code gen is like.
    //
    // struct Sample {
    //   a: i32,
    //   b: i64,
    // }
    //
    // from_conf_context(conf_context: &conf::ConfContext) -> Result<Self, Vec<conf::InnerError>> {
    //   let conf_context = #preprocess_conf_context;
    //   let mut errors = Vec::<conf::InnerError>::new();
    //
    //   let a = match || -> Result<i32, conf::InnerError> { ... }() {
    //     Ok(val) => Some(val),
    //     Err(err) => {
    //       errors.push(err);
    //       None
    //     }
    //   };
    //
    //   let b = match || -> Result<i64, conf::InnerError> { ... }() {
    //     Ok(val) => Some(val),
    //     Err(err) => {
    //       errors.push(err);
    //       None
    //     }
    //   };
    //
    //   let return_value = match (a, b) {
    //     (Some(a), Some(b)) => Ok(Self {
    //        a,
    //        b,
    //     }),
    //     _ => Err(errors),
    //   }?;
    //
    //   validation_predicate(&return_value).map_err(|err|
    // vec![conf::InnerError::validation(&conf_context.id, err)])?;
    //
    //   Ok(return_value)
    // }
    //
    // The list of let a, let b... is called #initializations
    // The match (a,b, ...) { ... } is called #return_value
    // The validation_predicate(...) part is called #apply_validation_predicate
    let conf_context_ident = Ident::new("__conf_context__", Span::call_site());
    let errors_ident = Ident::new("__errors__", Span::call_site());
    // For each field, intialize a local variable with Option<T> which is some if it worked and None
    // if there were errors. Push all errors into #errors_ident.
    let initializations: Vec<TokenStream> = fields
        .iter()
        .map(|field| -> Result<TokenStream, syn::Error> {
            let field_name = field.get_field_name();
            let field_type = field.get_field_type();
            let (initializer, returns_multiple_errors) =
                field.gen_initializer(&conf_context_ident)?;
            // The initializer is the portion e.g.
            // || -> Result<T, conf::InnerError> { ... }
            //
            // It returns `Result<T, Vec<conf::InnerError>>` if returns_multiple_errors is true, otherwise it's Result<T, conf::InnerError>
            // It is allowed to read #conf_context_ident but not modify it
            // We have to put it inside a locally defined fn so that it cannot modify the errors buffer etc.

            Ok(if returns_multiple_errors {
              quote! {
                fn #field_name(#conf_context_ident: &::conf::ConfContext<'_>) -> Result<#field_type, Vec<::conf::InnerError>> {
                    #initializer
                }
                let #field_name = match #field_name(&#conf_context_ident) {
                    Ok(val) => Some(val),
                    Err(errs) => {
                        #errors_ident.extend(errs);
                        None
                    }
                };
              }
            } else {
              quote! {
                fn #field_name(#conf_context_ident: &::conf::ConfContext<'_>) -> Result<#field_type, ::conf::InnerError> {
                    #initializer
                }
                let #field_name = match #field_name(&#conf_context_ident) {
                    Ok(val) => Some(val),
                    Err(err) => {
                        #errors_ident.push(err);
                        None
                   },
                };
              }
            })
        })
        .collect::<Result<Vec<_>, syn::Error>>()?;

    let field_names = fields
        .iter()
        .map(|field| field.get_field_name())
        .collect::<Vec<_>>();

    let return_value: TokenStream = quote! {
        match (#(#field_names),*) {
            (#(Some(#field_names)),*) => Ok( Self { #(#field_names),* } ),
            _ => Err(#errors_ident)
        }
    };

    let instance_ident = Ident::new("__instance__", Span::call_site());

    let validation_routine =
        struct_item.gen_validation_routine(&instance_ident, &conf_context_ident, fields)?;

    let struct_ident = struct_item.get_ident();
    let struct_pre_process_conf_context =
        struct_item.gen_pre_process_conf_context(&conf_context_ident)?;

    Ok(quote! {
        fn from_conf_context<'a>(#conf_context_ident: ::conf::ConfContext<'a>) -> Result<Self, Vec<::conf::InnerError>> {
            #struct_pre_process_conf_context

            let mut #errors_ident = Vec::<::conf::InnerError>::new();

            #(#initializations)*

            let return_value = #return_value?;

            fn validation<'a>(#instance_ident: & #struct_ident, #conf_context_ident: ::conf::ConfContext<'a>) -> Result<(), Vec<::conf::InnerError>> {
                #validation_routine
            }

            validation(&return_value, #conf_context_ident)?;

            Ok(return_value)
        }
    })
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
            let variants = collect_enum_variants(ident, variants.into_iter())?;
            gen_subcommands_impl_for_enum(ident, &input.generics, &variants)
        }

        _ => Err(Error::new(
            ident.span(),
            "#[derive(Subcommands)] is only supported on enums",
        )),
    }
}

fn gen_subcommands_impl_for_enum(
    item_name: &Ident,
    generics: &Generics,
    variants: &[VariantItem],
) -> Result<TokenStream, syn::Error> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let get_parsers_impl = gen_subcommands_get_parsers_impl_for_enum(item_name, variants)?;
    let get_subcommand_names_impl =
        gen_subcommands_get_subcommand_names_impl_for_enum(item_name, variants)?;
    let from_conf_context_impl =
        gen_subcommands_from_conf_context_impl_for_enum(item_name, variants)?;

    Ok(quote! {
        #[automatically_derived]
        #[allow(
            unused_qualifications,
        )]
        impl #impl_generics conf::Subcommands for #item_name #ty_generics #where_clause {
            #get_parsers_impl
            #get_subcommand_names_impl
            #from_conf_context_impl
        }
    })
}

fn gen_subcommands_get_parsers_impl_for_enum(
    _item_name: &Ident,
    variants: &[VariantItem],
) -> Result<TokenStream, syn::Error> {
    let parsers_ident = Ident::new("__parsers__", Span::call_site());
    let parsed_env_ident = Ident::new("__parsed_env__", Span::call_site());
    let variants_push_parsers: Vec<TokenStream> = variants
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

fn gen_subcommands_get_subcommand_names_impl_for_enum(
    _item_name: &Ident,
    variants: &[VariantItem],
) -> Result<TokenStream, syn::Error> {
    let command_names: Vec<_> = variants.iter().map(|var| var.get_command_name()).collect();

    Ok(quote! {
        fn get_subcommand_names() -> &'static [&'static str] {
            &[ #(#command_names,)* ]
        }
    })
}

fn gen_subcommands_from_conf_context_impl_for_enum(
    _item_name: &Ident,
    variants: &[VariantItem],
) -> Result<TokenStream, syn::Error> {
    let variant_match_arms: Vec<TokenStream> = variants
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
        fn from_conf_context(command_name: String, conf_context: ::conf::ConfContext<'_>) -> Result<Self, Vec<::conf::InnerError>> {
            match command_name.as_str() {
                #(#variant_match_arms,)*
                _ => { panic!("Unknown command name '{command_name}'. This is an internal error. Expected '{:?}'", <Self as ::conf::Subcommands>::get_subcommand_names()) }
            }
        }
    })
}
