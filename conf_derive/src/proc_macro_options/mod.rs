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
use syn::{parse_quote, Attribute, Error, FieldsNamed, Generics, Ident, LitStr, Token, Type};

mod field_item;
use field_item::FieldItem;

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
        // {   let conf_context = #preprocess_conf_context;
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

            fn validation<'ctxctx>(#instance_ident: & #struct_ident, #conf_context_ident: ::conf::ConfContext<'ctxctx>) -> Result<(), Vec<::conf::InnerError>> {
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

        // To generate a ConfSerde impl on S, we need to designate a Seed,
        // which will implement serde::DeserializeSeed.
        //
        // The Seed type can't exist within conf crate. If it did, then the
        // type is an conf, and the trait is in serde, so the impl would have to
        // be in conf, due to the orphan rules.
        // But the trait implementations need to be code-genned because they depend
        // on the user-defined type, and are going to live in the user's crate.
        // So, the Seed type needs to be a new-type of some kind, defined by this proc macro,
        // in the user's crate.
        //
        // In order to hide it, we define it in a "private module", and put the impl's there too:
        // const _: () = { ... };
        //
        // The Seed is just a newtype around ConfSerdeContext, which is what we would
        // have used if not for orphan rules.
        // (But actually, it needs phantom data pointing back to the user-type as well,
        // so that we can implement DeserializeSeed unambiguously. The user-type is an associated
        // type of the DeserializeSeed trait, and not a type parameter.)
        //
        // There are a few things we impl on the Seed:
        //
        // impl From<ConfSerdeContext> for Seed
        // impl Visitor<'de> for &Seed
        // impl DeserializeSeed<'de> for Seed
        //
        // Then we impl ConfSerde on Self, naming Seed as the associated type.
        //
        // * DeserializeSeed is the main workhorse here.
        // * From<ConfSerdeContext> is necessary so that the ConfSerde impl has a way to actually
        //   construct the seed.
        // * We do need something to impl Visitor, but it didn't have to be &Seed. It was just
        //   convenient to do it that way.

        let ident = self.struct_item.get_ident();
        let seed_ident = Ident::new("__SEED__", Span::call_site());

        let visitor_impl = self.gen_serde_visitor_impl(&seed_ident, generics)?;
        let deserialize_seed_impl = self.gen_serde_deserialize_seed_impl(&seed_ident, generics)?;

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
                use ::conf::{ConfSerdeContext, ConfSerde, InnerError, serde::de};

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

                #visitor_impl
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
    }

    /// Generate implementation of serde::Visitor for &Seed
    /// Panics if serde was not requested on this struct
    ///
    /// Arguments:
    /// * seed_ident is the identifier used in this scope for the Seed type
    /// * generics associated to this struct declaration
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
    }

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
        generics: &Generics,
    ) -> Result<TokenStream, Error> {
        let _serde_opts = self.struct_item.serde.as_ref().unwrap();

        // We need to add two generic lifetimes to the lifetime list (but only in the impl)
        // One is the "deserializer lifetime", and one is the "context lifetime".
        let de = make_lifetime("'dedede");
        let ct = make_lifetime("'ctctct");

        let seed_generics = prepend_generic_lifetimes(generics, [&ct]);
        let visitor_generics = prepend_generic_lifetimes(&seed_generics, [&de]);

        let (_, ty_generics, _) = generics.split_for_impl();
        let (impl_generics, _, _) = visitor_generics.split_for_impl();

        let ident = self.struct_item.get_ident();
        let ident_str = ident.to_string();

        let conf_context_ident = Ident::new("__conf_context__", Span::call_site());
        let errors_ident = Ident::new("__errors__", Span::call_site());

        let deserialize_finalizer_body =
            self.get_deserialize_finalizer_body(&conf_context_ident, &errors_ident)?;

        let field_names: Vec<&Ident> = self.fields.iter().map(|f| f.get_field_name()).collect();
        let field_name_strs: Vec<String> = field_names.iter().map(ToString::to_string).collect();

        // The Visitor impl Value type. This is a tuple containing Option< #field_type> and
        // Vec<InnerError>. It represents the partially finished work done using the
        // document values from serde.
        let visitor_tuple_type = self.gen_visitor_tuple_type();
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

                    use ::conf::{ConfContext, ConfSerde, ConfSerdeContext, InnerError};

                    fn __deserialize_finalizer(
                        #conf_context_ident: ConfContext<'_>,
                        (( #( mut #field_names,)* ), mut #errors_ident): #visitor_tuple_type
                    ) -> #value_type {
                        #deserialize_finalizer_body
                    }

                    Ok(match __deserializer.deserialize_struct(#ident_str, &[ #(#field_name_strs,)* ], &self) {
                        Ok(tuple_val) => __deserialize_finalizer(self.ctxt.conf_context, tuple_val),
                        Err(err) => {
                            let ctxt: ConfSerdeContext = self.ctxt;
                            Err(vec![ InnerError::serde(ctxt.document_name, #ident_str, err) ])
                        },
                    })
                }
            }
        })
    }

    // Body of the deserialize finalizer function
    // In this scope, all #field_name variables have type Option<Option<T>>,
    // and they are Some if the serde step encountered those fields, and None otherwise
    //
    // In this step, we initialize exactly those fields that were not initialized by the
    // serde walk.
    //
    // Arguments:
    // * conf_context_ident is the identifier of a ConfContext variable in scope, which we may
    //   consume
    // * errors_ident is the identifier of a mut Vec<InnerError> variable in scope, which we may
    //   consume
    fn get_deserialize_finalizer_body(
        &self,
        conf_context_ident: &Ident,
        errors_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        // For each field, #field_name is currently a local variable of type Option<Option<T>>.
        // If serde produced a value, then don't change anything.
        // Otherwise, use the initializer from the non-serde path.
        // We use `unwrap_or_else` here to accomplish this, and pass the initializer expr
        // inside a lambda function which prevents shadowing the let binding.
        // Push all errors into #errors_ident.
        let initializations: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|field| {
                let field_name = field.get_field_name();
                let initializer = field.gen_initialize_from_conf_context_and_push_errors(
                    conf_context_ident,
                    errors_ident,
                )?;

                Ok(quote! {
                    let #field_name = #field_name.unwrap_or_else(|| {
                        #initializer
                    });
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // Now, every variable has either been initialized by the serde path or the non serde path,
        // and if it is still None, it means there was an error. We can gather and validate as
        // usual.
        let gather_and_validate = self.gather_and_validate(conf_context_ident, errors_ident)?;

        Ok(quote! {
            #(#initializations)*

            #gather_and_validate
        })
    }
}
