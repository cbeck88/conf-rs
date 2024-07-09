use super::FieldItem;
use crate::util::*;
use proc_macro2::TokenStream;
use quote::quote;
use std::{cmp::Ordering, collections::HashMap};
use syn::{Attribute, Expr, Ident, LitStr};

/// #[conf(...)] options listed on a struct which has `#[derive(Conf)]`
pub struct StructItem {
    pub struct_ident: Ident,
    pub about: Option<LitStr>,
    pub name: Option<LitStr>,
    pub no_help_flag: bool,
    pub env_prefix: Option<LitStr>,
    pub one_of_fields: Vec<(Ordering, List<Ident>)>,
    pub validation_predicate: Option<Expr>,
    pub doc_string: Option<String>,
}

impl StructItem {
    /// Parse conf options out of attributes on a struct
    pub fn new(struct_ident: &Ident, attrs: &[Attribute]) -> Result<Self, syn::Error> {
        let mut result = Self {
            struct_ident: struct_ident.clone(),
            about: None,
            name: None,
            no_help_flag: false,
            env_prefix: None,
            one_of_fields: Vec::default(),
            validation_predicate: None,
            doc_string: None,
        };

        for attr in attrs {
            maybe_append_doc_string(&mut result.doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("no_help_flag") {
                        result.no_help_flag = true;
                        Ok(())
                    } else if path.is_ident("about") {
                        set_once(
                            &path,
                            &mut result.about,
                            Some(parse_required_value::<LitStr>(meta)?),
                        )
                    } else if path.is_ident("name") {
                        set_once(
                            &path,
                            &mut result.name,
                            Some(parse_required_value::<LitStr>(meta)?),
                        )
                    } else if path.is_ident("env_prefix") {
                        set_once(
                            &path,
                            &mut result.env_prefix,
                            Some(parse_required_value::<LitStr>(meta)?),
                        )
                    } else if path.is_ident("validation_predicate") {
                        set_once(
                            &path,
                            &mut result.validation_predicate,
                            Some(parse_required_value::<Expr>(meta)?),
                        )
                    } else if path.is_ident("one_of_fields") {
                        let idents: List<Ident> = meta.input.parse()?;
                        if idents.elements.len() < 2 {
                            return Err(meta.error(
                                "invalid to create a constraint over fewer than two fields",
                            ));
                        }
                        result.one_of_fields.push((Ordering::Equal, idents));
                        Ok(())
                    } else if path.is_ident("at_most_one_of_fields") {
                        let idents: List<Ident> = meta.input.parse()?;
                        if idents.elements.len() < 2 {
                            return Err(meta.error(
                                "invalid to create a constraint over fewer than two fields",
                            ));
                        }
                        result.one_of_fields.push((Ordering::Less, idents));
                        Ok(())
                    } else if path.is_ident("at_least_one_of_fields") {
                        let idents: List<Ident> = meta.input.parse()?;
                        if idents.elements.len() < 2 {
                            return Err(meta.error(
                                "invalid to create a constraint over fewer than two fields",
                            ));
                        }
                        result.one_of_fields.push((Ordering::Greater, idents));
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized conf option"))
                    }
                })?;
            }
        }

        Ok(result)
    }

    /// Get the identifier of this struct
    pub fn get_ident(&self) -> &Ident {
        &self.struct_ident
    }

    /// Generate a conf::ParserConfig expression, based on top-level options in this struct
    pub fn gen_parser_config(&self) -> Result<TokenStream, syn::Error> {
        // This default if name is not explicitly set matches what clap-derive does.
        let name = self
            .name
            .as_ref()
            .map(|lit_str| lit_str.value())
            .unwrap_or_else(|| std::env::var("CARGO_PKG_NAME").ok().unwrap_or_default());
        let no_help_flag = self.no_help_flag;
        let about_text = self
            .about
            .as_ref()
            .map(|lit_str| lit_str.value())
            .or(self.doc_string.clone());
        let about = quote_opt(&about_text);
        Ok(quote! {
            conf::ParserConfig {
                about: #about,
                name: #name,
                no_help_flag: #no_help_flag,
            }
        })
    }

    /// Generate an (optional) program options post-processing step.
    /// If we have an env_prefix at struct-level, apply it here.
    pub fn gen_post_process_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<Option<TokenStream>, syn::Error> {
        if self.env_prefix.is_none() {
            return Ok(None);
        }

        let apply_flatten_prefixes = self
            .env_prefix
            .as_ref()
            .map(|env_prefix| quote! { .apply_flatten_prefixes("", "", #env_prefix, "") });

        Ok(Some(quote! {
            #program_options_ident = #program_options_ident.into_iter().map(
              |opt| opt
                #apply_flatten_prefixes
            ).collect();
        }))
    }

    /// Generate an (optional) ConfContext pre-processing step.
    pub fn gen_pre_process_conf_context(
        &self,
        _conf_context_ident: &Ident,
    ) -> Result<Option<TokenStream>, syn::Error> {
        Ok(None)
    }

    /// Generate tokens that apply any validations to an instance
    ///
    /// These tokens are the body of a validation function with signature
    /// fn validation(#instance_ident: &Self, #instance_id_prefix_ident: &str) -> Result<(), Vec<conf::InnerError>>
    pub fn gen_validation_routine(
        &self,
        instance: &Ident,
        conf_context_ident: &Ident,
        fields: &[FieldItem],
    ) -> Result<TokenStream, syn::Error> {
        let struct_ident = &self.struct_ident;
        let struct_name = self.struct_ident.to_string();
        let mut predicate_evaluations = Vec::<TokenStream>::new();
        let mut fields_helper = FieldsHelper::new(instance, conf_context_ident, fields);

        for (ordering, list) in &self.one_of_fields {
            let count_expr = fields_helper.make_count_expr_for_field_list(list)?;
            // Split into single options, with ids (relative to this prefix), and flattened structs.
            let (id_list, flattened_list): (Vec<String>, Vec<Ident>) =
                fields_helper.split_single_options_and_flattened(list)?;

            // Depending on ordering parameter, a count of 0 is either okay or an error
            let zero_arm = if *ordering == Ordering::Less {
                quote! { Ok(()) }
            } else {
                let quoted_flattened_id_list = flattened_list
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<String>>();
                quote! { Err(#conf_context_ident.too_few_arguments_error(#struct_name, &[#(#id_list),*], &[#(#quoted_flattened_id_list),*])) }
            };

            // Depending on ordering parameter, a count of > 1 is either okay or an error
            let more_than_one_arm = if *ordering == Ordering::Greater {
                quote! { Ok(()) }
            } else {
                let quoted_flattened_id_and_value_source_list = flattened_list
                    .iter()
                    .map(|ident| -> Result<TokenStream, syn::Error> {
                        let field_name = ident.to_string();
                        let get_value_source_expr =
                            fields_helper.make_get_value_source_expr(ident)?;
                        Ok(quote! {
                            (#field_name, #get_value_source_expr? )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                // Note: A lambda is used here because it's allowed that get_value_source_expr can fail and early return with ?, but this isn't really expected to happen.
                // The functions that it is calling will all be failing earlier in the process if they fail at all.
                quote! {
                    {
                        let flattened_ids_and_value_sources: Result<Vec<(&'static str, Option<(&str, ::conf::ConfValueSource::<&str>)>)>, ::conf::InnerError> =
                            (|| Ok(vec![#(#quoted_flattened_id_and_value_source_list),*]))();
                        match flattened_ids_and_value_sources {
                            Ok(flattened_ids_and_value_sources) => Err(#conf_context_ident.too_many_arguments_error(#struct_name, &[#(#id_list),*], flattened_ids_and_value_sources)),
                            Err(err) => Err(err)
                        }
                    }
                }
            };

            // Push code which evaluates the predicate, returning Ok(()) or an Inner Error.
            predicate_evaluations.push(quote! {
                {
                    let count: u32 = #count_expr;
                    match count {
                        0 => #zero_arm,
                        1 => Ok(()),
                        _ => #more_than_one_arm,
                    }
                }
            });
        }

        // Apply user-provided validation predicate, if any
        if let Some(user_validation_predicate) = self.validation_predicate.as_ref() {
            predicate_evaluations.push(quote! {
                {
                    fn __validation_predicate__(#instance: & #struct_ident) -> Result<(), impl ::core::fmt::Display> {
                        #user_validation_predicate(#instance)
                    }
                    __validation_predicate__(#instance).map_err(|err| ::conf::InnerError::validation(#struct_name, & #conf_context_ident .get_id_prefix(), err))
                }
            });
        }

        // Collect all predicate evluations, and aggregate their errors.
        Ok(if predicate_evaluations.is_empty() {
            quote! {
                Ok(())
            }
        } else {
            quote! {
               let errors = [#(#predicate_evaluations),*].into_iter().filter_map(|result| result.err()).collect::<Vec<::conf::InnerError>>();
               if errors.is_empty() {
                   Ok(())
               } else {
                   Err(errors)
               }
            }
        })
    }
}

// struct which caches lookup from ident to FieldItem, and generates tokenstreams for checking if these fields are present in the struct instance etc.
struct FieldsHelper<'a> {
    instance: &'a Ident,
    conf_context_ident: &'a Ident,
    fields: &'a [FieldItem],
    cache: HashMap<Ident, &'a FieldItem>,
}

impl<'a> FieldsHelper<'a> {
    pub fn new(
        instance: &'a Ident,
        conf_context_ident: &'a Ident,
        fields: &'a [FieldItem],
    ) -> Self {
        Self {
            instance,
            conf_context_ident,
            fields,
            cache: Default::default(),
        }
    }

    pub fn get_field(&mut self, ident: &Ident) -> Result<&'a FieldItem, syn::Error> {
        let field_item = if let Some(val) = self.cache.get(ident) {
            val
        } else {
            let field = self
                .fields
                .iter()
                .find(|field| field.get_field_name() == ident)
                .ok_or_else(|| syn::Error::new(ident.span(), "identifier not found in struct"))?;
            self.cache.insert(ident.clone(), field);
            field
        };
        Ok(field_item)
    }

    pub fn get_is_present_expr(&mut self, ident: &Ident) -> Result<TokenStream, syn::Error> {
        let field_item = self.get_field(ident)?;
        let field_type = field_item.get_field_type();
        let instance = &self.instance;

        if let FieldItem::Parameter(item) = field_item {
            if item.get_default_value().is_some() {
                return Err(syn::Error::new(ident.span(), "using one_of_fields constraint with a field that has a default_value is invalid, since it will always be present."));
            }
        };

        let tok = if type_is_bool(&field_type) {
            quote! { #instance.#ident }
        } else if type_is_option(&field_type)?.is_some() {
            quote! { #instance.#ident.is_some() }
        } else if type_is_vec(&field_type)?.is_some() {
            quote! { !#instance.#ident.is_empty() }
        } else {
            return Err(syn::Error::new(
                ident.span(),
                "field must be bool, Option<T>, or Vec<T> to use with one_of_fields constraint",
            ));
        };

        Ok(tok)
    }

    pub fn make_count_expr_for_field_list(
        &mut self,
        list: &List<Ident>,
    ) -> Result<TokenStream, syn::Error> {
        let u32_exprs: Vec<TokenStream> = list
            .elements
            .iter()
            .map(|ident| -> Result<TokenStream, syn::Error> {
                let bool_expr = self.get_is_present_expr(ident)?;
                Ok(quote! { #bool_expr as u32 })
            })
            .collect::<Result<_, _>>()?;
        Ok(quote! {
            #(#u32_exprs)+*
        })
    }

    pub fn split_single_options_and_flattened(
        &mut self,
        list: &List<Ident>,
    ) -> Result<(Vec<String>, Vec<Ident>), syn::Error> {
        let mut single_opts = Vec::<String>::new();
        let mut groups = Vec::<Ident>::new();

        for ident in &list.elements {
            let field_item = self.get_field(ident)?;
            if field_item.is_single_option() {
                single_opts.push(ident.to_string());
            } else {
                groups.push(ident.clone());
            }
        }
        Ok((single_opts, groups))
    }

    pub fn make_get_value_source_expr(&mut self, ident: &Ident) -> Result<TokenStream, syn::Error> {
        let field_item = self.get_field(ident)?;
        match field_item {
            FieldItem::Flatten(flatten_item) => {
                Ok(flatten_item.any_program_options_appeared_expr(self.conf_context_ident)?)
            }
            _ => Err(syn::Error::new(
                ident.span(),
                "field is not flattened, this is an internal error",
            )),
        }
    }
}
