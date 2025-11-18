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
use syn::{Attribute, Error, FieldsNamed, Generics, Ident, LitStr, Type};

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
    pub fn new(ident: &Ident, attrs: &[Attribute], fields: &FieldsNamed) -> Result<Self, Error> {
        let struct_item = StructItem::new(ident, attrs)?;
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
    ///
    /// Takes generics associated to the struct.
    pub fn gen_conf_impl(&self, generics: &Generics) -> Result<TokenStream, Error> {
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
        let struct_name = self.struct_item.get_ident().to_string();

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

    /// Generate Conf::get_program_options implementation
    fn get_program_options_impl(&self) -> Result<TokenStream, Error> {
        // To implement Conf::get_program_options, we need to
        // get all the program options for our constituents. To do this, we create
        // an ident for the list of program options, which is going to be Vec<ProgramOption>.
        // Then we pass that ident to every constitutent field, and aggregate all their code gen.
        let program_options_ident = Ident::new("__program_options__", Span::call_site());
        let fields_push_program_options: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|field| field.gen_push_program_options(&program_options_ident))
            .collect::<Result<Vec<_>, Error>>()?;

        // To implement #[conf(env_prefix="ACME_")] on a struct (rather than on a flattened field),
        // the code gen associated to the struct needs to be able to add its own prefixing during
        // get_program_options and during from_conf_context.
        // To do this, we allow the struct_item to "post-process" the Vec<ProgramOption>, (to add a
        // prefix to them all) and to "pre-process" the ConfContext (to add a matching prefix to
        // that before it is used) Note: The preprocessing no longer does anything since we switched
        // to using id's like clap does.
        let struct_post_process_program_options = self
            .struct_item
            .gen_post_process_program_options(&program_options_ident)?;

        // Note: fields_push_program_options is allowed to early return with ? on an error
        Ok(quote! {
            fn get_program_options() -> Result<&'static [::conf::ProgramOption], ::conf::Error> {
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
        //   validation_predicate(&return_value).map_err(|err| {
        //     vec![conf::InnerError::validation(&conf_context.id, err)]
        //   })?;
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
                let field_name = field.get_field_name();
                let initializer = field.gen_initialize_from_conf_context_and_push_errors(
                    &conf_context_ident,
                    &errors_ident,
                )?;
                Ok(quote! {
                    let #field_name = #initializer;
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let gather_and_validate = self.gather_and_validate(&conf_context_ident, &errors_ident)?;

        Ok(quote! {
            fn from_conf_context<'a>(#conf_context_ident: ::conf::ConfContext<'a>) -> Result<Self, Vec<::conf::InnerError>> {
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

        let field_names: Vec<&Ident> = self
            .fields
            .iter()
            .map(|field| field.get_field_name())
            .collect();

        let return_value: TokenStream = quote! {
            match (#(#field_names),*) {
                (#(Some(#field_names)),*) => #struct_ident { #(#field_names),* },
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

            fn validation<'ctxctx>(#instance_ident: & #struct_ident, #conf_context_ident: &::conf::ConfContext<'ctxctx>) -> Result<(), Vec<::conf::InnerError>> {
                #validation_routine
            }

            validation(&return_value, #conf_context_ident)?;

            Ok(return_value)
        })
    }

    /// Generate an impl ConfSerde block for this struct (if requested via attributes)
    /// Also, the requisite DeserializeSeed impl's and such.
    ///
    /// Takes generics associated to this struct.
    pub fn maybe_gen_conf_serde_impl(
        &self,
        generics: &Generics,
    ) -> Result<Option<TokenStream>, Error> {
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

        let struct_ident = self.struct_item.get_ident();
        let struct_ident_str = struct_ident.to_string();
        let expecting_str = format!("Object with schema {struct_ident}");

        let machine_ident = Ident::new("__MACHINE__", Span::call_site());
        let machine = self.gen_machine(&machine_ident, generics)?;

        // These generics are used to impl ConfSerde on the user's type.
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        Ok(Some(quote! {
            const _: () = {
                use ::core::{fmt, option::Option, marker::PhantomData, result::Result};
                use ::std::vec::Vec;
                use ::conf::{ConfSerdeContext, ConfSerde, ConfSerdeSeed, InnerError, NextValueProducer, SubcommandsSerde, serde::de::{self, Error}};

                #machine

                impl #impl_generics ConfSerde for #struct_ident #ty_generics #where_clause {
                    type ISM = #machine_ident #ty_generics;

                    const STRUCT_NAME: &str = #struct_ident_str;
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
    fn gen_machine(
        &self,
        machine_ident: &Ident,
        generics: &Generics,
    ) -> Result<TokenStream, Error> {
        let struct_ident = &self.struct_item.struct_ident;
        let struct_ident_str = struct_ident.to_string();
        let serde_opts = self.struct_item.serde.as_ref().unwrap();

        let conf_serde_context_ident = Ident::new("__conf_serde_context__", Span::call_site());
        let errors_ident = Ident::new("__errors__", Span::call_site());
        let nvp_ident = Ident::new("__nvp__", Span::call_site());
        let nvp_type_ident = Ident::new("NVP__", Span::call_site());

        let (_, ty_generics, _) = generics.split_for_impl();

        // One is the "deserializer lifetime", and one is the "context lifetime".
        let ct = make_lifetime("'ctctct");
        let de = make_lifetime("'dedede");

        let visitor_generics = prepend_generic_lifetimes(generics, [&de]);
        let (impl_visitor_generics, _, _) = visitor_generics.split_for_impl();

        let field_names: Vec<&Ident> = self.fields.iter().map(|f| f.get_field_name()).collect();
        let serde_strategies = self
            .fields
            .iter()
            .map(|f| {
                f.gen_serde_strategy(
                    &conf_serde_context_ident,
                    &nvp_ident,
                    &nvp_type_ident,
                    &errors_ident,
                )
            })
            .collect::<Result<Vec<SerdeStrategy>, _>>()?;

        // The machine has member variables of the form #field_name: Option<#field_machine_type>
        let field_machine_types: Vec<&Type> = serde_strategies
            .iter()
            .map(|s| &s.state_machine_type)
            .collect();

        // These match arms are used to implement `next`
        let field_match_arms: Vec<&TokenStream> =
            serde_strategies.iter().map(|s| &s.match_arm).collect();

        // This is the catch-all arm in the match statement
        let handle_unknown_field = if !serde_opts.allow_unknown_fields {
            let serde_help_names = serde_strategies
                .iter()
                .flat_map(|s| match &s.serde_help_keys {
                    SerdeKeys::Lit(v) => v.iter(),
                    SerdeKeys::Expr(_) => [].iter(),
                })
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

        // This is the implementation of the keys function which *MUST* match the patterns declared
        // in the field match arms.
        let serde_keys: Vec<&SerdeKeys> = serde_strategies.iter().map(|s| &s.serde_keys).collect();
        let keys_fn_body = if serde_keys.iter().any(|sk| matches!(sk, SerdeKeys::Expr(_))) {
            let tokens: Vec<TokenStream> = serde_keys
                .iter()
                .map(|sk| match sk {
                    SerdeKeys::Lit(v) => quote! { [#(#v,)*].iter() },
                    SerdeKeys::Expr(x) => quote! { #x },
                })
                .collect();

            quote! {
              static KEYS: ::std::sync::OnceLock<Vec<&'static str>> = ::std::sync::OnceLock::new();
              KEYS.get_or_init(|| {
                let mut result = vec![];
                #(result.extend(#tokens);)*
                result
              })
            }
        } else {
            let lit_keys = serde_keys
                .iter()
                .flat_map(|sk| match sk {
                    SerdeKeys::Lit(v) => v.iter(),
                    SerdeKeys::Expr(_) => [].iter(),
                })
                .collect::<Vec<&LitStr>>();

            quote! {
              &[ #(#lit_keys,)* ]
            }
        };

        // Pick out the names of fields that need a call to finalizer to finalize their state machine,
        // along with the context expression to use for each.
        let finalizer_statements: Vec<TokenStream> = field_names
            .iter()
            .zip(serde_strategies.iter())
            .filter_map(|(n, s)| {
                s.finalizer_context.as_ref().map(|ctx| {
                    quote! {
                        let #n = #n.map(|m| match m.finalize(#ctx) {
                            Ok(val) => Some(val),
                            Err(err) => { #errors_ident.extend(err); None }
                        });
                    }
                })
            })
            .collect();

        // Expressions that initialize things without serde, in case serde traversal doesn't ever produce
        // this field.
        let conf_context_ident = Ident::new("__conf_context__", Span::call_site());
        let fallback_initializers: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|field| {
                field.gen_initialize_from_conf_context_and_push_errors(
                    &conf_context_ident,
                    &errors_ident,
                )
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let gather_and_validate = self.gather_and_validate(&conf_context_ident, &errors_ident)?;

        Ok(quote! {
            pub struct #machine_ident #ty_generics {
                #errors_ident: Vec<InnerError>,
                #(#field_names: Option<#field_machine_types>),*
            };

            impl #generics ::core::default::Default for #machine_ident #ty_generics {
                fn default() -> Self {
                    #(let #field_names: Option<#field_machine_types> = None;)*

                    Self {
                        #errors_ident: Default::default(),
                        #(#field_names,)*
                    }
                }
            }

            impl #impl_visitor_generics ::conf::InitializationStateMachine<#de> for #machine_ident #ty_generics {
                type Value = #struct_ident #ty_generics;
                type Context<#ct> = ConfSerdeContext<#ct>;

                fn keys() -> &'static[&'static str] {
                    #keys_fn_body
                }

                fn next<#ct, #nvp_type_ident>(self, __key__: &str, #nvp_ident: #nvp_type_ident, #conf_serde_context_ident: &Self::Context<#ct>) -> Self
                where #nvp_type_ident: NextValueProducer<#de>
                {
                    let Self {
                        mut #errors_ident,
                        #(mut #field_names,)*
                    } = self;

                    'match_statement: {
                        match __key__ {
                            #(#field_match_arms)*
                            __other__ => { #handle_unknown_field }
                        }
                    }

                    Self {
                        #errors_ident,
                        #(#field_names,)*
                    }
                }

                fn finalize<#ct>(self, #conf_serde_context_ident: &Self::Context<#ct>) -> Result<Self::Value, Vec<InnerError>> {
                    let Self {
                        mut #errors_ident,
                        #(#field_names,)*
                    } = self;

                    // Finalize all state machines, collecting any errors.
                    #(#finalizer_statements)*

                    // If anything hasn't been initialized by serde, try to initialize with
                    // the no-serde code path.
                    let #conf_context_ident = &#conf_serde_context_ident.conf_context;
                    #(let #field_names = #field_names.unwrap_or_else(|| { #fallback_initializers });)*

                    // Now, every variable has either been initialized by the serde path or the non serde path,
                    // and if it is still None, it means there was an error. We can gather and validate as
                    // usual.
                    #gather_and_validate
                }
            }
        })
    }

    /// Generate Conf::debug_asserts implementation
    fn debug_asserts_impl(&self) -> Result<TokenStream, Error> {
        let struct_ident = self.struct_item.get_ident();
        let assertions: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|field| field.gen_debug_asserts(struct_ident))
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(quote! {
            fn debug_asserts() {
                #(#assertions)*
            }
        })
    }

    /// Generate a test function (if requested via #[conf(test)] attribute)
    pub fn maybe_gen_test_fn(&self, generics: &Generics) -> Result<Option<TokenStream>, Error> {
        // If test is not requested, don't generate anything
        let test_item = match &self.struct_item.test {
            Some(test_item) => test_item,
            None => return Ok(None),
        };

        let ident = self.struct_item.get_ident();
        let test_fn_name = Ident::new(&format!("conf_debug_assert_{}", ident), ident.span());

        // Check if we have generics - if so, we can't easily generate a test
        // because we don't know what concrete types to use
        if !generics.params.is_empty() {
            return Err(Error::new(
                ident.span(),
                "#[conf(test)] cannot be used with generic types yet",
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
