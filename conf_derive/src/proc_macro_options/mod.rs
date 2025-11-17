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
use syn::{Attribute, Error, FieldsNamed, Generics, Ident, LitStr, Type, parse_quote};

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
    // * conf_context_ident: The identifier of a ConfContext variable that we can use (and consume)
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

            validation(&return_value, &#conf_context_ident)?;

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

        // To generate a ConfSerde impl on S, we need to designate a Seed,
        // which will implement serde::DeserializeSeed.
        //
        // In modern versions of the crate, we create a new type M
        // M, where M: InitializationStateMachine<Value = S> + From<ConfSerdeContext>.
        //
        // This object intuitively represents the state that is maintained in
        // iterations of the loop over a `serde::de::MapAccess`:
        // while Some(key) = ma.next_key()? {
        //   match key {
        //      field1 => { ... },
        //      field2 => { ... },
        //   }
        // }
        //
        // M is the workhorse, but it isn't actually defined to be the seed.
        //
        // Instead the seed is a new-type wrapper around ConfSerdeContext,
        // and it implements DeserializeSeed in terms of M.
        //
        // In order to hide this type M, we define it in a "private module", and put the impl's there too:
        // const _: () = { ... };
        //

        let ident = self.struct_item.get_ident();
        let machine_ident = Ident::new("__MACHINE__", Span::call_site());
        let seed_ident = Ident::new("__SEED__", Span::call_site());

        let machine = self.gen_machine(&machine_ident, generics)?;
        let deserialize_seed_impl =
            self.gen_serde_deserialize_seed_impl(&seed_ident, &machine_ident, generics)?;

        // These generics are used to impl ConfSerde on the user's type.
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        // The Seed struct needs an additional lifetime (the conf context lifetime)
        // These generics are used when declaring the seed and implementing traits on it.
        let ct = make_lifetime("'ctctct");
        let seed_generics = prepend_generic_lifetimes(generics, [&ct]);
        let (seed_impl_generics, seed_ty_generics, seed_where_clause) =
            seed_generics.split_for_impl();

        Ok(Some(quote! {
            const _: () = {
                use ::core::{fmt, option::Option, marker::PhantomData, result::Result};
                use ::std::vec::Vec;
                use ::conf::{ConfSerdeContext, ConfSerde, InnerError, NextValueProducer, SubcommandsSerde, serde::de::{self, Error}};

                #machine

                pub struct #seed_ident #seed_generics {
                    ctxt: ConfSerdeContext<#ct>,
                    marker: PhantomData<fn() -> #ident #ty_generics>,
                };

                impl #seed_impl_generics From<ConfSerdeContext<#ct>> for #seed_ident #seed_ty_generics #seed_where_clause {
                    fn from(ctxt: ConfSerdeContext<#ct>) -> Self {
                        Self {
                            ctxt,
                            marker: Default::default(),
                        }
                    }
                }

                #deserialize_seed_impl

                impl #impl_generics ConfSerde for #ident #ty_generics #where_clause {
                    type Seed<#ct> = #seed_ident #seed_generics;
                }
            };
        }))
    }

    // Helper which generates the tuple type used as the serde::Visitor::Value,
    // the "output type" of the visitor.
    //
    // ( ( Option< Option< #field_type > >... ), Vec<InnerError> )
    //
    // For a given #field_name,
    // * None means serde did not produce this key and so we still have to visit it without serde
    //   afterwards
    // * Some(None) means serde visited it and it produced an error.
    // * Some(Some(val)) means serde visited it and produced a value.
    /*
    fn gen_visitor_tuple_type(&self) -> Type {
        let field_types: Vec<Type> = self.fields.iter().map(|f| f.get_field_type()).collect();
        // ( #ty ) is not a tuple type in rust, it must be ( #ty , ) when the tuple size is one.
        let extra_comma = if field_types.len() == 1 {
            Some(<Token![,]>::default())
        } else {
            None
        };
        parse_quote! {
            ( ( #( Option< Option< #field_types > > ),* #extra_comma) , Vec<InnerError> )
        }
    }*/

    /// Generate implementation of serde::Visitor for &Seed
    /// Panics if serde was not requested on this struct
    ///
    /// Arguments:
    /// * seed_ident is the identifier used in this scope for the Seed type
    /// * generics associated to this struct declaration
    /*
    fn gen_serde_visitor_impl(
        &self,
        seed_ident: &Ident,
        generics: &Generics,
    ) -> Result<TokenStream, Error> {
        let serde_opts = self.struct_item.serde.as_ref().unwrap();

        // We need to add two generic lifetimes to the lifetime list (but only in the impl)
        // One is the "deserializer lifetime", and one is the "context lifetime".
        let de = make_lifetime("'dedede");
        let ct = make_lifetime("'ctctct");

        let seed_generics = prepend_generic_lifetimes(generics, [&ct]);
        let visitor_generics = prepend_generic_lifetimes(&seed_generics, [&de]);

        let (impl_generics, _, _) = visitor_generics.split_for_impl();

        let ident = self.struct_item.get_ident();
        let expecting_str = format!("Object with schema {ident}");
        let ident_str = ident.to_string();

        let conf_serde_context_ident = Ident::new("__conf_serde_context__", Span::call_site());
        let errors_ident = Ident::new("__errors__", Span::call_site());
        let map_access_ident = Ident::new("__map_access__", Span::call_site());
        let map_access_type = Ident::new("MA__", Span::call_site());

        let field_names: Vec<&Ident> = self.fields.iter().map(|f| f.get_field_name()).collect();
        let field_match_arms_and_serde_names = self
            .fields
            .iter()
            .map(|f| {
                f.gen_serde_match_arm(
                    &conf_serde_context_ident,
                    &map_access_ident,
                    &map_access_type,
                    &errors_ident,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (field_match_arms, serde_names): (Vec<TokenStream>, Vec<Vec<LitStr>>) =
            field_match_arms_and_serde_names.into_iter().unzip();
        let serde_names: Vec<LitStr> = serde_names.into_iter().flatten().collect();

        // The Visitor impl Value type. This is a tuple containing Option< #field_type> and
        // Vec<InnerError> It represents the partially finished work done using the document
        // values from serde.
        let visitor_tuple_type = self.gen_visitor_tuple_type();
        // ( #id ) is not a tuple in rust, it must be ( #id , ) when the tuple size is one.
        let extra_comma = if field_names.len() == 1 {
            Some(<Token![,]>::default())
        } else {
            None
        };

        let handle_unknown_field = if !serde_opts.allow_unknown_fields {
            Some(quote! {
                #errors_ident.push(
                   InnerError::serde(
                     #conf_serde_context_ident.document_name,
                     #ident_str,
                     #map_access_type::Error::unknown_field(__other__, &[ #(#serde_names),* ])
                   )
                );
            })
        } else {
            None
        };

        Ok(quote! {
            impl #impl_generics de::Visitor<#de> for &#seed_ident #seed_generics {
                type Value = #visitor_tuple_type;

                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, #expecting_str)
                }

                fn visit_map<#map_access_type>(self, mut #map_access_ident: #map_access_type) -> Result<Self::Value, #map_access_type::Error>
                    where #map_access_type: de::MapAccess<#de>
                {
                    use ::conf::{ConfSerdeContext, IdentString, InnerError, SubcommandsSerde, serde::de::Error};
                    let #conf_serde_context_ident: &ConfSerdeContext = &self.ctxt;
                    let ( ( #(mut #field_names),* #extra_comma), mut #errors_ident ) = Self::Value::default();

                    while let Some(key) = #map_access_ident.next_key::<IdentString>()? {
                        match key.as_str() {
                            #(#field_match_arms)*
                            __other__ => { #handle_unknown_field }
                        }
                    }

                    Ok( ( ( #(#field_names),* #extra_comma), #errors_ident ) )
                }
            }
        })
    }*/

    // Generate an implementation of serde::DeserializerSeed on Seed
    // Panics if serde was not requested on this struct
    //
    // *** How does this work? ***
    //
    // In the basic, no-serde version, things are pretty simple.
    // Each field has an "initializer" expression, generated by FieldItem,
    // which can init the field (from conf context) or return errors.
    // We do `let #field_name: Option<#field_type> = #initializer`; for each field.
    // Then we have a local variable for every field. (And a conf context and an error buffer in
    // scope.)
    //
    // Then we use `gather_and_validate`.
    // If any errors occurred, we return the entire error buffer.
    // If not, we can essentially unwrap all the #field_name variables, because every initializer
    // expression either returns Some(#field_type) or produces an error.
    // Then we move them into the struct type.
    // Then we try to run any validation routines. If any of them fail, return all the errors.
    // Otherwise we succeeded and can return the struct.
    //
    // Each #initalizer is responsible for checking all the possible value sources and making the
    // priority work, and applying value_parser etc. A lot of that logic is in the `ConfContext`
    // methods, but it's resolved independently on a per field basis.
    //
    // Any ordering of running the #initializers would be fine, since they are independent of
    // eachother. We just happen to run them in the order that they were declared.
    //
    // ---
    //
    // Once serde is in the mix, things are a bit more complicated, because, we don't get to decide
    // the order in which serde gives us values. The MapAccess object gives us the keys and
    // values in whatever order it wants.
    //
    // Instead, what we do is, we make a serde Visitor which uses the ConfContext, and walks
    // whichever of the fields serde MapAccess produces a value for. Then it returns
    // `Option<Option<#field_type>>` and `Vec<InnerError>`, which go on the stack.
    //
    // The field is:
    //   None if `serde` did not attempt to visit it
    //   Some(None) if `serde` visited it, but an error resulted
    //   Some(Some(...)) if `serde successfully produced a value.
    //
    // (The distinction between None and Some(None) is important because, it's possible that args or
    // env supplies a String for  a given field, but the ValueParser produces errors, but also
    // that `serde` produces multiple value for the same key, which  is independently a separate
    // error. We don't ever want to run a ValueParser twice.  For the same reason, if serde
    // visited a field and it produced an error, we want to remember that and not visit it in round
    // 2  in the post-serde phase. It's confusing if we try to deserialize the same field in two
    // different ways and potentially  get two different errors.)
    // Then we do a modified version of the no-serde initialization routine -- we check which
    // options are populated (were given a value by serde), and any that are not populated, we
    // run the no-serde initializer for. At the end, either everything is populated or we have
    // at least one error. We run the same gather_and_validate routine to return the resulting
    // struct.
    //
    // Arguments:
    // * seed_ident is the identifier used in this scope for the Seed type
    // * generics associated to this struct declaration
    fn gen_serde_deserialize_seed_impl(
        &self,
        seed_ident: &Ident,
        machine_ident: &Ident,
        generics: &Generics,
    ) -> Result<TokenStream, Error> {
        let ident = &self.struct_item.struct_ident;
        let expecting_str = format!("Object with schema {ident}");
        let ident_str = ident.to_string();

        // We need to add two generic lifetimes to the lifetime list (but only in the impl)
        // One is the "deserializer lifetime", and one is the "context lifetime".
        let de = make_lifetime("'dedede");
        let ct = make_lifetime("'ctctct");

        let seed_generics = prepend_generic_lifetimes(generics, [&ct]);
        let visitor_generics = prepend_generic_lifetimes(&seed_generics, [&de]);

        let (_, ty_generics, _) = generics.split_for_impl();
        let (impl_generics, _, _) = visitor_generics.split_for_impl();

        // The Visitor impl Value type. This is a tuple containing Option< #field_type> and
        // Vec<InnerError>. It represents the partially finished work done using the
        // document values from serde.
        // let visitor_tuple_type = self.gen_visitor_tuple_type();
        // The type that will be the Value of this DeserializeSeed impl
        let value_type: Type = parse_quote! {
            Result<#ident #ty_generics, Vec<InnerError>>
        };

        // High level:
        //
        // To implement deserialize seed, we take the deserializer, and call deserialize_struct.
        // We pass a reference to ourself as the visitor, which was implemented in
        // gen_serde_visitor_impl.
        //
        // This produces a Visitor::Value, which is a tuple consisting of
        // Option<Option<#field_type>>, indicating which fields we successfully
        // deserialized, which ones tried to deserialize and failed, and which ones were not
        // attempted in the serde phase. The __deserialize_finalizer finishes the job,
        // initializing everything that serde didn't attempt to initialize, and then running
        // validators etc.
        //
        // It may also produce a serde::de::Error. See gen_serde_visitor_impl -- that only happens
        // if there is an error getting the next key from the map.
        //
        // If there is an error getting the next key from the map, it means that the deserializer
        // data is not very well-formed -- most likely, the user is not iterating
        // serde_json::Value or serde_yaml::Value or similar, because that should not give
        // such an error. There may be a bunch of other key-value pairs that are in the
        // user's data file, but are not parseable due to a syntax issue.
        //
        // If we continue trying to initialize stuff, we're likely going to get nonsense -- missing
        // field errors for all the subsequent keys in this map that serde could not read.
        //
        // We'd rather only report the root cause -- that we couldn't iterate the map properly.
        // So we bail out in that case, and don't attempt to proceed with finalizing.
        Ok(quote! {
            impl #impl_generics de::DeserializeSeed<#de> for #seed_ident #seed_generics {
                type Value = #value_type;

                fn deserialize<D__>(self, __deserializer: D__) -> Result<Self::Value, D__::Error>
                    where D__: de::Deserializer<#de> {

                    fn expecting_fn(f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        write!(f, #expecting_str)
                    }

                    Ok(::conf::deserialize_seed_impl::<#ct, #de, D__, #machine_ident>(#ident_str, expecting_fn, self.ctxt, __deserializer))
                }
            }
        })
    }

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

        let ct = make_lifetime("'ctctct");

        //let seed_generics = prepend_generic_lifetimes(generics, [&ct]);
        //let (impl_from_generics, _, _) = seed_generics.split_for_impl();

        let de = make_lifetime("'dedede");
        let visitor_generics = prepend_generic_lifetimes(&generics, [&de]);
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

        // Pick out the names of fields that need a call to finalizer to finalize their state machine
        let fields_with_finalizers: Vec<&Ident> = field_names
            .iter()
            .zip(serde_strategies.iter())
            .filter(|(_n, s)| s.has_finalizer)
            .map(|(n, _s)| *n)
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
            struct #machine_ident #ty_generics {
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
                    #(let #fields_with_finalizers = #fields_with_finalizers.and_then(|m| match m.finalize(#conf_serde_context_ident) { Ok(val) => Some(val), Err(err) => { #errors_ident.extend(err); None }});)*

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
