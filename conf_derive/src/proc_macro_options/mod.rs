//! GenConfStruct helps with parsing syn data for a Conf Struct, and generating Conf trait
//! implementation bits.
//!
//! This module also provides StructItem, FieldItem helper structures which:
//! * Parse the `#[conf(...)]` attributes that appear on different types of items
//! * Store the results and make them easily available
//! * Assist with subsequent codegen

use crate::util::{make_lifetime, prepend_generic_lifetimes};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, Error, FieldsNamed, Generics, Ident, LitStr, Type, Visibility, parse_quote};

mod field_item;
use field_item::{FieldItem, SerdeKeys, SerdeStrategy};

mod struct_item;
use struct_item::StructItem;

/// Helper which generates individual functions related to `#[derive(Conf)]`
/// on a struct.
///
/// Calling "new" parses all the proc macro attributes for struct and fields.
/// Calling individual functions returns code gen.
pub struct GenConfStruct {
    struct_item: StructItem,
    fields: Vec<FieldItem>,
}

impl GenConfStruct {
    /// Parse syn data for a struct with derive(Conf) on it
    pub fn new(
        ident: &Ident,
        vis: &Visibility,
        generics: &Generics,
        attrs: &[Attribute],
        fields: &FieldsNamed,
    ) -> Result<Self, Error> {
        let struct_item = StructItem::new(ident, vis, generics, attrs)?;
        let fields = fields
            .named
            .iter()
            .map(|f| FieldItem::new(f, &struct_item))
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            struct_item,
            fields,
        })
    }

    /// Generate an impl Conf block for this struct
    pub fn gen_conf_impl(&self) -> Result<TokenStream, Error> {
        let generics = &self.struct_item.generics.raw;
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let ident = self.struct_item.get_ident();
        let conf_fns = vec![
            self.get_parser_config_impl()?,
            self.get_program_options_impl()?,
            self.get_subcommands_impl()?,
            self.from_conf_context_impl()?,
            self.get_name_impl()?,
            self.debug_asserts_impl()?,
        ];

        Ok(quote! {
            #[automatically_derived]
            #[allow(
                unused_qualifications,
            )]
            impl #impl_generics ::conf::Conf for #ident #ty_generics #where_clause {
                #(#conf_fns)*
            }
        })
    }

    /// Generate Conf::get_name implementation
    fn get_name_impl(&self) -> Result<TokenStream, Error> {
        let struct_name = self.struct_item.get_display_name();

        Ok(quote! {
            fn get_name() -> &'static str {
                #struct_name
            }
        })
    }

    /// Generate Conf::get_parser_config implementation
    fn get_parser_config_impl(&self) -> Result<TokenStream, Error> {
        // To implement Conf::get_parser_config, we need to get a ParserConfig object
        // for this struct, (top-level config essentially).
        let parser_config = self.struct_item.gen_parser_config()?;

        Ok(quote! {
            fn get_parser_config() -> Result<::conf::ParserConfig, ::conf::Error> {
                let parser_config = #parser_config;

                Ok(parser_config)
            }
        })
    }

    /// Generate Conf::PROGRAM_OPTIONS implementation
    fn get_program_options_impl(&self) -> Result<TokenStream, Error> {
        // Generate Node<ProgramOption> for each field
        // Field types that don't contribute (e.g., Subcommands) return None and are filtered out
        let field_nodes: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|field| field.gen_program_option_node(&self.struct_item.generics))
            .collect::<Result<Vec<_>, Error>>()?
            .into_iter()
            .flatten()
            .collect();

        // Get the transform function for struct-level prefixing if needed
        let struct_transform = self.struct_item.gen_program_options_transform()?;

        Ok(quote! {
            // Note: not a `static`, since a static can't use the struct's generics, but a const's
            // borrowed array is still promoted to 'static.
            const PROGRAM_OPTIONS: ::conf::lazybuf::LazyBuf<::conf::ProgramOption> =
                ::conf::lazybuf::LazyBuf {
                    buffer: &[
                        #(#field_nodes),*
                    ],
                    transform: #struct_transform,
                };
        })
    }

    /// Generate Conf::get_subcommands implementation
    fn get_subcommands_impl(&self) -> Result<TokenStream, Error> {
        let parsers_ident = Ident::new("__parsers__", Span::call_site());
        let parsed_env_ident = Ident::new("__parsed_env__", Span::call_site());
        let fields_push_subcommands: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|field| field.gen_push_subcommands(&parsers_ident, &parsed_env_ident))
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(quote! {
            fn get_subcommands(#parsed_env_ident: &::conf::ParsedEnv) -> Result<Vec<::conf::Parser>, ::conf::Error> {
                let mut #parsers_ident = vec![];

                #(#fields_push_subcommands)*

                Ok(#parsers_ident)
            }
        })
    }

    // Generate Conf::from_conf_context implementation
    #[allow(clippy::wrong_self_convention)]
    fn from_conf_context_impl(&self) -> Result<TokenStream, Error> {
        // To implement Conf::from_conf_context, we need to take a conf context,
        // and then return Ok(Self { ... }). For each constituent field, we need it
        // to generate code to initialize itself properly. We pass the ConfContext ident
        // to each constituent field, and then aggregate all their code gen.
        // Their code-gen is allowed to use `?` or `return Err(...)` to early return,
        // but we still need to aggregate all the errors. Sample code gen is like.
        //
        // struct Sample {
        //   a: i32,
        //   b: i64,
        // }
        //
        // from_conf_context(conf_context: conf::ConfContext) -> Result<Self, Vec<conf::InnerError>>
        // {
        //   let mut errors = Vec::<conf::InnerError>::new();
        //
        //   // If the struct is generic, each nested fn like this re-declares its generics, and is
        //   // called with a turbofish, e.g. a::<T>(&conf_context). See NestedGenerics.
        //   fn a(conf_context: &conf::ConfContext) -> Result<i32, conf::InnerError> {
        //      ..
        //   }
        //   let a = match a(&conf_context) {
        //     Ok(val) => Some(val),
        //     Err(err) => {
        //       errors.push(err);
        //       None
        //     }
        //   };
        //
        //   fn b(conf_context: &conf::ConfContext) -> Result<i64, conf::InnerError> {
        //      ..
        //   }
        //   let b = match b(&conf_context) {
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
        //   let validation = |instance: &Sample| -> Result<(), Vec<conf::InnerError>> {
        //     // e.g. check validation predicates, using the captured conf_context
        //   };
        //   validation(&return_value)?;
        //
        //   Ok(return_value)
        // }
        //
        // The list of let a, let b... is called #initializations
        // The match (a,b, ...) { ... } is called #return_value
        // The validation_predicate(...) part is called #apply_validation_predicate
        let conf_context_ident = Ident::new("__conf_context__", Span::call_site());
        let errors_ident = Ident::new("__errors__", Span::call_site());

        // For each field, intialize a local variable with Option<T> which is some if it worked and
        // None if there were errors. Push all errors into #errors_ident.
        let initializations: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|field| -> Result<TokenStream, Error> {
                let field_name = field.get_local_ident();
                let initializer = field.gen_initialize_from_conf_context_and_push_errors(
                    &conf_context_ident,
                    &errors_ident,
                    &self.struct_item.generics,
                )?;
                Ok(quote! {
                    let #field_name = #initializer;
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let gather_and_validate = self.gather_and_validate(&conf_context_ident, &errors_ident)?;

        Ok(quote! {
            fn from_conf_context(#conf_context_ident: ::conf::ConfContext<'_>) -> Result<Self, Vec<::conf::InnerError>> {
                let mut #errors_ident = Vec::<::conf::InnerError>::new();

                // Rebind as reference so #conf_context_ident has consistent type &ConfContext
                let #conf_context_ident = &#conf_context_ident;

                #(#initializations)*

                #gather_and_validate
            }
        })
    }

    // Generate a routine which gathers struct fields
    // (local variables represented as Option<#field type>).
    //
    // Bails if we can't produce a struct, or if the constituted struct fails validation.
    // Otherwise returns it.
    //
    // Arguments:
    // * conf_context_ident: The identifier of a &ConfContext in scope
    // * errors_ident: the identifier of a `mut Vec<InnerError>` buffer variable which is in scope.
    fn gather_and_validate(
        &self,
        conf_context_ident: &Ident,
        errors_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        let struct_ident = self.struct_item.get_ident();
        let struct_ty_generics = &self.struct_item.generics.ty;

        let field_names: Vec<&Ident> = self
            .fields
            .iter()
            .map(|field| field.get_field_name())
            .collect();
        let field_locals: Vec<Ident> = self.fields.iter().map(|f| f.get_local_ident()).collect();

        let return_value: TokenStream = quote! {
            match (#(#field_locals),*) {
                (#(Some(#field_locals)),*) => #struct_ident { #(#field_names: #field_locals),* },
                _ => panic!("Internal error: no errors encountered but struct was incomplete")
            }
        };

        let instance_ident = Ident::new("__instance__", Span::call_site());

        let validation_routine = self.struct_item.gen_validation_routine(
            &instance_ident,
            conf_context_ident,
            &self.fields,
        )?;

        Ok(quote! {
            if !#errors_ident.is_empty() {
                return Err(#errors_ident);
            }

            let return_value = #return_value;

            // A closure rather than a nested fn, since a nested fn can't use the struct's generics.
            let validation = |#instance_ident: & #struct_ident #struct_ty_generics| -> Result<(), Vec<::conf::InnerError>> {
                #validation_routine
            };

            validation(&return_value)?;

            Ok(return_value)
        })
    }

    /// Generate an impl ConfSerde block for this struct (if requested via attributes)
    /// Also, the requisite DeserializeSeed impl's and such.
    pub fn maybe_gen_conf_serde_impl(&self) -> Result<Option<TokenStream>, Error> {
        let generics = &self.struct_item.generics.raw;

        // If serde is not requested, fugeddaboutit
        if self.struct_item.serde.is_none() {
            return Ok(None);
        };

        // To implement ConfSerde, the main goal is to generate an initialization
        // state machine. This becomes an associated type on the ConfSerde implementation
        // for the target type.
        //
        // In order to hide this type M, we define it in a "private module", and put the impl's there too:
        // const _: () = { ... };
        //
        // M must have the same visibility as the struct. M is the associated type ISM of the
        // struct's ConfSerde impl, and the struct is the associated type Value of M's
        // InitializationStateMachine impl, so if either is less visible than the other, it's
        // a private type in a public interface (E0446). A const block is not a module, so the
        // struct's visibility means the same thing on M inside it.

        let struct_ident = self.struct_item.get_ident();
        let struct_ident_str = struct_ident.to_string();
        let expecting_str = format!("Object with schema {struct_ident}");

        let machine_ident = Ident::new("__MACHINE__", Span::call_site());
        let (machine, struct_keys) = self.gen_machine(&machine_ident)?;

        // These generics are used to impl ConfSerde on the user's type.
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        // The machine has a context lifetime in it
        let ct = make_lifetime("'ctctct");
        let machine_generics = prepend_generic_lifetimes(generics, [&ct]);
        let (_, machine_ty_generics, _) = machine_generics.split_for_impl();

        Ok(Some(quote! {
            const _: () = {
                use ::core::{fmt, option::Option, marker::PhantomData, result::Result};
                use ::std::vec::Vec;
                use ::conf::{ConfSerdeContext, ConfSerde, ConfSerdeSeed, InnerError, NextValueProducer, SubcommandsSerde, serde::de::{self, Error}};

                #machine

                impl #impl_generics ConfSerde for #struct_ident #ty_generics #where_clause {
                    type ISM<#ct> = #machine_ident #machine_ty_generics;

                    const STRUCT_NAME: &str = #struct_ident_str;
                    const STRUCT_KEYS: Option<&[&str]> = #struct_keys;

                    fn expecting(f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        write!(f, #expecting_str)
                    }
                }
            };
        }))
    }

    /// Create an object implementing InitializationStateMachine whose value is this struct,
    /// and whose context is ConfSerdeContext.
    ///
    /// The machine contains a field for each field of the struct, where each field value is
    /// Option<#field_machine_type>. Field machine type is *usually* Option<T>.
    /// However for more complex cases, like additional flattened structs, it might not be.
    ///
    /// It also contains a buffer of errors called `__errors__`.
    /// The machine implements Default.
    ///
    /// In order to implement the state machine trait we need to do three things:
    /// * Compute all the keys that we are interested, as a static list.
    /// * Implement the "next" function which takes one key value pair and advances
    ///   one state machine of one of our fields, or stores an unknown value error.
    /// * Implement the "finalize" function, which must produce our target value
    ///   or yield one or more errors.
    ///
    /// To implement keys, we ask our constituents what their keys are and aggregating them.
    ///
    /// To implement next, we match on the key and send it to one of the constituents.
    /// Each constituent generates their own match arm for us.
    ///
    /// To implement finalize, we:
    /// * Call finalize on any state machines, which moves those types from Option<#field_machine_type> to Option<Option<T>>.
    /// * For anything that is still None, serde never produced a value. Therefore we have to try to populate it using the
    ///   non-serde route. This is done using .unwrap_or_else, so the type becomes Option<T>. The "fallback initializers"
    ///   perform this task.
    /// * Finally we call gather_and_validate, simliar to the non-serde route.
    ///
    /// The first token stream contains the machine definition and impls.
    /// The second token stream is the value for STRUCT_KEYS for this struct.
    fn gen_machine(&self, machine_ident: &Ident) -> Result<(TokenStream, TokenStream), Error> {
        let generics = &self.struct_item.generics.raw;
        let struct_ident = &self.struct_item.struct_ident;
        let struct_vis = &self.struct_item.struct_vis;
        let struct_ident_str = struct_ident.to_string();
        let serde_opts = self.struct_item.serde.as_ref().unwrap();

        let conf_serde_context_ident = Ident::new("__conf_serde_context__", Span::call_site());
        let errors_ident = Ident::new("__errors__", Span::call_site());
        let nvp_ident = Ident::new("__nvp__", Span::call_site());
        let nvp_type_ident = Ident::new("NVP__", Span::call_site());

        // A generic struct's machine needs a PhantomData, in case some generic param isn't used
        // by any field's machine type. Non-generic structs leave it out, so it can't clash with
        // one of their field names.
        let (phantom_decl, phantom_init, phantom_bind) = if self.struct_item.generics.is_empty() {
            Default::default()
        } else {
            let phantom_ident = Ident::new("__phantom__", Span::call_site());
            let phantom_ty = &self.struct_item.generics.phantom;
            (
                quote! { #phantom_ident: #phantom_ty, },
                quote! { #phantom_ident: PhantomData, },
                quote! { #phantom_ident, },
            )
        };

        let (_, ty_generics, _) = generics.split_for_impl();

        // One is the "deserializer lifetime", and one is the "context lifetime".
        let ct = make_lifetime("'ctctct");
        let de = make_lifetime("'dedede");

        // machine is generic over ct since it contains a context
        let machine_generics = prepend_generic_lifetimes(generics, [&ct]);
        let (impl_machine_generics, machine_ty_generics, machine_where_clause) =
            machine_generics.split_for_impl();

        // Used when we impl state machine trait
        let visitor_generics = prepend_generic_lifetimes(&machine_generics, [&de]);
        let (impl_visitor_generics, _, _) = visitor_generics.split_for_impl();

        // The machine's fields are named like the struct's fields, but are bound to locals with
        // different names (see field_local_ident).
        let field_names: Vec<&Ident> = self.fields.iter().map(|f| f.get_field_name()).collect();
        let field_locals: Vec<Ident> = self.fields.iter().map(|f| f.get_local_ident()).collect();
        let serde_strategies = self
            .fields
            .iter()
            .map(|f| {
                f.gen_serde_strategy(
                    &ct,
                    &conf_serde_context_ident,
                    &nvp_ident,
                    &nvp_type_ident,
                    &errors_ident,
                    &self.struct_item.generics,
                )
            })
            .collect::<Result<Vec<SerdeStrategy>, _>>()?;

        // The machine has member variables of the form #field_name: #field_machine_type
        let field_machine_types: Vec<Type> = self
            .fields
            .iter()
            .zip(serde_strategies.iter())
            .map(|(field, strat)| {
                strat.state_machine_type.clone().unwrap_or_else(|| {
                    let field_type = field.get_field_type();
                    parse_quote! { Option<Option<#field_type>> }
                })
            })
            .collect();

        // The machines are initialized by these initializer expressions.
        let field_machine_initializers: Vec<TokenStream> = serde_strategies
            .iter()
            .map(|strat| {
                strat
                    .state_machine_init
                    .clone()
                    .unwrap_or_else(|| quote! { None })
            })
            .collect();

        // These match arms are used to implement `wants_key`
        let field_match_patterns: Vec<TokenStream> = serde_strategies
            .iter()
            .filter_map(|s| s.serde_keys.gen_match_pattern())
            .collect();

        // These match arms are used to implement `next`
        let field_match_arms: Vec<TokenStream> = serde_strategies
            .iter()
            .filter_map(|s| {
                let match_pattern = s.serde_keys.gen_match_pattern()?;
                let match_expr = &s.match_expr;
                Some(quote! { #match_pattern => #match_expr })
            })
            .collect();

        // This is the catch-all expr in the match statement
        let handle_unknown_field = if !serde_opts.allow_unknown_fields {
            let serde_help_names = serde_strategies
                .iter()
                .filter_map(|s| s.serde_keys.help_key())
                .collect::<Vec<&LitStr>>();
            Some(quote! {
                #errors_ident.push(
                   InnerError::serde(
                     #conf_serde_context_ident.document_name,
                     #struct_ident_str,
                     #nvp_type_ident::Error::unknown_field(__other__, &[ #(#serde_help_names),* ])
                   )
                );
            })
        } else {
            None
        };

        // Pick out the names of fields that need a call to finalizer to finalize their state machine,
        // along with the context expression to use for each.
        // For fields without an actual state machine, we include expressions that initialize things
        // without serde, in case serde traversal doesn't ever produce this field.
        let conf_context_ident = Ident::new("__conf_context__", Span::call_site());
        let finalizer_statements: Vec<TokenStream> = self
            .fields
            .iter()
            .zip(serde_strategies.iter())
            .map(|(field, strat)| {
                let n = field.get_local_ident();
                Ok(if strat.state_machine_type.is_some() {
                    quote! {
                        let #n = match #n.finalize() {
                            Ok(val) => Some(val),
                            Err(err) => { #errors_ident.extend(err); None }
                        };
                    }
                } else {
                    let fallback_initializer = field
                        .gen_initialize_from_conf_context_and_push_errors(
                            &conf_context_ident,
                            &errors_ident,
                            &self.struct_item.generics,
                        )?;
                    quote! {
                        let #n = #n.unwrap_or_else(|| #fallback_initializer);
                    }
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let gather_and_validate = self.gather_and_validate(&conf_context_ident, &errors_ident)?;

        let machine = quote! {
            // Declared with the full generics (bounds, const params) and where clause, not just
            // the parameter names, so const params stay const and field types that need the
            // bounds are well-formed.
            #struct_vis struct #machine_ident #impl_machine_generics #machine_where_clause {
                #conf_serde_context_ident: ConfSerdeContext<#ct>,
                #errors_ident: Vec<InnerError>,
                #phantom_decl
                #(#field_names: #field_machine_types),*
            }

            impl #impl_machine_generics From<ConfSerdeContext<#ct>> for #machine_ident #machine_ty_generics #machine_where_clause {
                fn from(#conf_serde_context_ident: ConfSerdeContext<#ct>) -> Self {
                    #(let #field_locals: #field_machine_types = #field_machine_initializers;)*

                    Self {
                        #conf_serde_context_ident,
                        #errors_ident: Default::default(),
                        #phantom_init
                        #(#field_names: #field_locals,)*
                    }
                }
            }

            impl #impl_visitor_generics ::conf::InitializationStateMachine<#de> for #machine_ident #machine_ty_generics #machine_where_clause {
                type Value = #struct_ident #ty_generics;

                #[allow(unused)]
                fn wants_key(&self, __key__: &str) -> bool {
                    #(let #field_locals = &self.#field_names;)*

                    match __key__ {
                        #(#field_match_patterns => true,)*
                        _ => false,
                    }
                }

                fn next<#nvp_type_ident>(self, __key__: &str, #nvp_ident: #nvp_type_ident) -> Self
                  where #nvp_type_ident: NextValueProducer<#de>
                {
                    let Self {
                        #conf_serde_context_ident,
                        mut #errors_ident,
                        #phantom_bind
                        #(#field_names: mut #field_locals,)*
                    } = self;

                    'match_statement: {
                        match __key__ {
                            #(#field_match_arms)*
                            __other__ => { #handle_unknown_field }
                        }
                    }

                    Self {
                        #conf_serde_context_ident,
                        #errors_ident,
                        #phantom_bind
                        #(#field_names: #field_locals,)*
                    }
                }

                fn finalize(self) -> Result<Self::Value, Vec<InnerError>> {
                    let Self {
                        #conf_serde_context_ident,
                        mut #errors_ident,
                        #phantom_bind
                        #(#field_names: #field_locals,)*
                    } = self;

                    let #conf_context_ident = &#conf_serde_context_ident.conf_context;
                    // Finalize all state machines and non-state machines, collecting any errors.
                    #(#finalizer_statements)*

                    // Now, every variable has either been initialized by the serde path or the non serde path,
                    // and if it is still None, it means there was an error. We can gather and validate as
                    // usual.
                    #gather_and_validate
                }

                fn needs_finalize(&self) -> bool {
                    // Check if any program options from this struct appeared in the conf context
                    // (from args or env). This is used for flatten-optional to determine if the
                    // group should be activated when serde doesn't mention it.
                    <Self::Value as ::conf::Conf>::any_program_options_appeared(
                        &self.#conf_serde_context_ident.conf_context
                    ).unwrap_or(None).is_some()
                }
            }
        };

        let struct_keys = if let Some(all_keys) =
            SerdeKeys::all_literal_keys(serde_strategies.iter().map(|s| &s.serde_keys))
        {
            quote! { Some(&[ #(#all_keys),* ]) }
        } else {
            quote! { None }
        };

        Ok((machine, struct_keys))
    }

    /// Generate Conf::debug_asserts implementation
    fn debug_asserts_impl(&self) -> Result<TokenStream, Error> {
        let struct_ident = self.struct_item.get_ident();
        let assertions: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|field| field.gen_debug_asserts(struct_ident, &self.struct_item.generics))
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(quote! {
            fn debug_asserts() {
                // Check for short-form conflicts at this level
                {
                    let mut short_forms = ::std::collections::HashMap::<char, String>::new();
                    for opt in Self::PROGRAM_OPTIONS.iter() {
                        if let Some(short) = opt.short_form {
                            if let Some(existing_id) = short_forms.insert(short, opt.id.to_string()) {
                                panic!(
                                    "Short option '{}' is used by both '{}' and '{}' in {}",
                                    short,
                                    existing_id,
                                    opt.id,
                                    stringify!(Self)
                                );
                            }
                        }
                    }
                }

                #(#assertions)*
            }
        })
    }

    /// Generate a test function (if requested via #[conf(test)] attribute)
    pub fn maybe_gen_test_fn(&self) -> Result<Option<TokenStream>, Error> {
        // If test is not requested, don't generate anything
        let test_item = match &self.struct_item.test {
            Some(test_item) => test_item,
            None => return Ok(None),
        };

        let ident = self.struct_item.get_ident();
        let test_fn_name = Ident::new(&format!("conf_debug_assert_{}", ident), ident.span());

        // Check if we have generics - if so, we can't easily generate a test
        // because we don't know what concrete types to use
        if !self.struct_item.generics.is_empty() {
            return Err(Error::new(
                ident.span(),
                "#[conf(test)] cannot be used on a generic struct. Put it on a non-generic struct that uses this one with concrete types, e.g. via flatten.",
            ));
        }

        // Optionally add #[should_panic] attribute
        let maybe_should_panic = if test_item.should_panic {
            quote! { #[should_panic] }
        } else {
            quote! {}
        };

        Ok(Some(quote! {
            #[cfg(test)]
            #[test]
            #maybe_should_panic
            #[allow(non_snake_case)]
            fn #test_fn_name() {
                <#ident as ::conf::Conf>::parser_debug_asserts();
                <#ident as ::conf::Conf>::debug_asserts();
            }
        }))
    }
}
