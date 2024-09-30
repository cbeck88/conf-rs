use super::StructItem;
use crate::util::*;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    meta::ParseNestedMeta, parse_quote, spanned::Spanned, token, Error, Expr, Field, Ident,
    LitBool, LitChar, LitStr, Type,
};

/// #[conf(serde(...))] options listed on a field of Repeat kind
pub struct RepeatSerdeItem {
    pub rename: Option<LitStr>,
    pub skip: bool,
    pub use_value_parser: bool,
    span: Span,
}

impl RepeatSerdeItem {
    pub fn new(meta: ParseNestedMeta<'_>) -> Result<Self, Error> {
        let mut result = Self {
            rename: None,
            skip: false,
            use_value_parser: false,
            span: meta.input.span(),
        };

        if meta.input.peek(token::Paren) {
            meta.parse_nested_meta(|meta| {
                let path = meta.path.clone();
                if path.is_ident("rename") {
                    set_once(
                        &path,
                        &mut result.rename,
                        Some(parse_required_value::<LitStr>(meta)?),
                    )
                } else if path.is_ident("skip") {
                    result.skip = true;
                    Ok(())
                } else if path.is_ident("use_value_parser") {
                    result.use_value_parser = true;
                    Ok(())
                } else {
                    Err(meta.error("unrecognized conf(serde) option"))
                }
            })?;
        }

        Ok(result)
    }
}

impl GetSpan for RepeatSerdeItem {
    fn get_span(&self) -> Span {
        self.span
    }
}

/// Proc macro annotations parsed from a field of Repeat kind
pub struct RepeatItem {
    field_name: Ident,
    field_type: Type, // This is needed to help with type inference in code gen
    allow_hyphen_values: bool,
    secret: Option<LitBool>,
    long_switch: Option<LitStr>,
    aliases: Option<LitStrArray>,
    env_name: Option<LitStr>,
    env_aliases: Option<LitStrArray>,
    value_parser: Option<Expr>,
    env_delimiter: Option<LitChar>,
    no_env_delimiter: bool,
    serde: Option<RepeatSerdeItem>,
    description: Option<String>,
}

impl RepeatItem {
    pub fn new(field: &Field, _struct_item: &StructItem) -> Result<Self, Error> {
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "missing identifier"))?;
        let field_type = field.ty.clone();

        let Some(inner_type) = type_is_vec(&field_type)? else {
            return Err(Error::new(
                field.ty.span(),
                "Type of a conf(repeat) field must be Vec<T>",
            ));
        };
        let allow_hyphen_values = type_is_signed_number(&inner_type);

        let mut result = Self {
            field_name,
            field_type,
            allow_hyphen_values,
            secret: None,
            long_switch: None,
            aliases: None,
            env_name: None,
            env_aliases: None,
            value_parser: None,
            env_delimiter: None,
            no_env_delimiter: false,
            serde: None,
            description: None,
        };

        for attr in &field.attrs {
            maybe_append_doc_string(&mut result.description, &attr.meta)?;
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("repeat") {
                        Ok(())
                    } else if path.is_ident("long") {
                        set_once(
                            &path,
                            &mut result.long_switch,
                            parse_optional_value::<LitStr>(meta)?
                                .or(make_long(&result.field_name, path.span())),
                        )
                    } else if path.is_ident("aliases") {
                        set_once(
                            &path,
                            &mut result.aliases,
                            Some(parse_required_value::<LitStrArray>(meta)?),
                        )
                    } else if path.is_ident("env") {
                        set_once(
                            &path,
                            &mut result.env_name,
                            parse_optional_value::<LitStr>(meta)?
                                .or(make_env(&result.field_name, path.span())),
                        )
                    } else if path.is_ident("env_aliases") {
                        set_once(
                            &path,
                            &mut result.aliases,
                            Some(parse_required_value::<LitStrArray>(meta)?),
                        )
                    } else if path.is_ident("value_parser") {
                        set_once(
                            &path,
                            &mut result.value_parser,
                            Some(parse_required_value::<Expr>(meta)?),
                        )
                    } else if path.is_ident("env_delimiter") {
                        set_once(
                            &path,
                            &mut result.env_delimiter,
                            Some(parse_required_value::<LitChar>(meta)?),
                        )
                    } else if path.is_ident("no_env_delimiter") {
                        result.no_env_delimiter = true;
                        Ok(())
                    } else if path.is_ident("allow_hyphen_values") {
                        result.allow_hyphen_values = true;
                        Ok(())
                    } else if path.is_ident("secret") {
                        set_once(
                            &path,
                            &mut result.secret,
                            Some(
                                parse_optional_value::<LitBool>(meta)?
                                    .unwrap_or(LitBool::new(true, path.span())),
                            ),
                        )
                    } else if path.is_ident("serde") {
                        set_once(&path, &mut result.serde, Some(RepeatSerdeItem::new(meta)?))
                    } else {
                        Err(meta.error("unrecognized conf repeat option"))
                    }
                })?;
            }
        }

        if result.no_env_delimiter && result.env_delimiter.is_some() {
            return Err(Error::new(
                field.span(),
                "Cannot specify both env_delimiter and no_env_delimiter",
            ));
        }

        if result.env_delimiter.is_some() && result.env_name.is_none() {
            return Err(Error::new(
                field.span(),
                "env_delimiter has no effect if an env variable is not declared",
            ));
        }

        if result.no_env_delimiter && result.env_name.is_none() {
            return Err(Error::new(
                field.span(),
                "no_env_delimiter has no effect if an env variable is not declared",
            ));
        }

        if result.long_switch.is_none()
            && !result
                .aliases
                .as_ref()
                .map(LitStrArray::is_empty)
                .unwrap_or(true)
        {
            return Err(Error::new(
                field.span(),
                "Setting aliases without setting a long-switch is an error, \
                make one of the aliases the primary switch name.",
            ));
        }

        if result.env_name.is_none()
            && !result
                .env_aliases
                .as_ref()
                .map(LitStrArray::is_empty)
                .unwrap_or(true)
        {
            return Err(Error::new(
                field.span(),
                "Setting env_aliases without setting an env is an error, \
                make one of the aliases the primary env.",
            ));
        }

        Ok(result)
    }

    pub fn get_field_name(&self) -> &Ident {
        &self.field_name
    }

    pub fn get_field_type(&self) -> Type {
        self.field_type.clone()
    }

    pub fn get_serde_name(&self) -> LitStr {
        self.serde
            .as_ref()
            .and_then(|serde| serde.rename.clone())
            .unwrap_or_else(|| LitStr::new(&self.field_name.to_string(), self.field_name.span()))
    }

    pub fn get_serde_type(&self) -> Type {
        let use_value_parser = self
            .serde
            .as_ref()
            .map(|serde| serde.use_value_parser)
            .unwrap_or(false);

        if use_value_parser {
            parse_quote! { ::std::vec::Vec<::std::string::String> }
        } else {
            self.field_type.clone()
        }
    }

    pub fn get_serde_skip(&self) -> bool {
        self.serde.as_ref().map(|serde| serde.skip).unwrap_or(false)
    }

    /// Generate a routine that pushes a ::conf::ProgramOption corresponding to
    /// this field, onto a mut Vec<ProgramOption> that is in scope.
    ///
    /// Arguments:
    /// * program_options_ident is the ident of this buffer of ProgramOption to push to.
    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        let id = self.field_name.to_string();
        let description = quote_opt_into(&self.description);
        let long_form = quote_opt_into(&self.long_switch);
        let aliases = self.aliases.as_ref().map(LitStrArray::quote_elements_into);
        let env_form = quote_opt_into(&self.env_name);
        let env_aliases = self
            .env_aliases
            .as_ref()
            .map(LitStrArray::quote_elements_into);
        let allow_hyphen_values = self.allow_hyphen_values;
        let secret = quote_opt(&self.secret);

        Ok(quote! {
            #program_options_ident.push(::conf::ProgramOption {
              id: #id.into(),
              parse_type: ::conf::ParseType::Repeat,
              description: #description,
              short_form: None,
              long_form: #long_form,
              aliases: vec![#aliases],
              env_form: #env_form,
              env_aliases: vec![#env_aliases],
              default_value: None,
              is_required: false,
              allow_hyphen_values: #allow_hyphen_values,
              secret: #secret,
            });
        })
    }

    pub fn gen_push_subcommands(
        &self,
        _subcommands_ident: &Ident,
        _parsed_env: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        Ok(quote! {})
    }

    fn get_delimiter(&self) -> TokenStream {
        quote_opt(&if self.no_env_delimiter {
            None
        } else {
            Some(
                self.env_delimiter
                    .clone()
                    .unwrap_or_else(|| LitChar::new(',', self.field_name.span())),
            )
        })
    }

    fn get_value_parser(&self) -> Expr {
        // Value parser is FromStr::from_str if not specified
        self.value_parser
            .clone()
            .unwrap_or_else(|| parse_quote! { std::str::FromStr::from_str })
    }

    fn gen_initializer_helper(
        &self,
        conf_context_ident: &Ident,
        before_value_parser: Option<TokenStream>,
    ) -> Result<(TokenStream, bool), syn::Error> {
        let field_type = &self.field_type;
        let id = self.field_name.to_string();

        let delimiter = self.get_delimiter();
        let value_parser = self.get_value_parser();

        // Note: We can't use rust into_iter, collect, map_err because sometimes it messes with type
        // inference around the value parser
        //
        // Note: The line `let mut result: #field_type = Default::default();`
        // is expected to be default initializing a Vec.
        // It can't be `#field_type::default()` because it requires turbofish syntax.
        //
        // If it fails because the user put another funky type there, imo this should not really be
        // supported. It's more compelling to make the value_parser option easier to use
        // (easier type inference) than to support user-defined containers here, and try to
        // use `.collect` etc. directly into their container. The user's code can do
        // .iter().collect() after our code runs if they want.
        let initializer = quote! {
          {
            fn __value_parser__(
              __arg__: &str
            ) -> Result<<#field_type as ::conf::InnerTypeHelper>::Ty, impl ::core::fmt::Display> {
              #value_parser(__arg__)
            }

            use ::conf::{ConfValueSource, ProgramOption, InnerError};
            use ::std::vec::Vec;

            let (value_source, strs, opt): (ConfValueSource<&str>, Vec<&str>, &ProgramOption)
              = #conf_context_ident.get_repeat_opt(#id, #delimiter).map_err(|err| vec![err])?;

            #before_value_parser

            let mut result: #field_type = Default::default();
            let mut errors = Vec::<InnerError>::new();
            result.reserve(strs.len());
            for val_str in strs {
              match __value_parser__(val_str) {
                Ok(val) => result.push(val),
                Err(err) => errors.push(
                  InnerError::invalid_value(
                    value_source.clone(),
                    val_str,
                    opt,
                    err.to_string()
                  )
                ),
              }
            }
            if errors.is_empty() {
              Ok(result)
            } else {
              Err(errors)
            }
          }
        };
        Ok((initializer, true))
    }

    // Gen initializer
    //
    // Create an expression that returns initialized #field_type value, or errors.
    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        self.gen_initializer_helper(conf_context_ident, None)
    }

    // Gen initializer with a provided doc val
    //
    // This should work similar to gen_initializer, but if the conf context produces a default
    // value, we should return the doc_val instead because it has higher priority than default,
    // but lower than args and env.
    pub fn gen_initializer_with_doc_val(
        &self,
        conf_context_ident: &Ident,
        doc_name: &Ident,
        doc_val: &Ident,
    ) -> Result<(TokenStream, bool), Error> {
        let use_value_parser = self
            .serde
            .as_ref()
            .map(|serde| serde.use_value_parser)
            .unwrap_or(false);

        if use_value_parser {
            // When use_value_parser is enabled, the behavior is, if conf_context produced a default
            // value, we should overwrite it with the document value. `val_strs` is a `Vec<&str>`,
            // and #doc_val is a `Vec<String>`.
            let before_value_parser = quote! {
              let (value_source, strs) = if value_source.is_default() {
                (ConfValueSource::Document(#doc_name), #doc_val.iter().map(String::as_str).collect())
              } else {
                (value_source, strs)
              };
            };

            self.gen_initializer_helper(conf_context_ident, Some(before_value_parser))
        } else {
            // When use_value_parser is not enabled, the behavior is, if conf context produced a
            // default value, we should instead simply return the doc value.
            let before_value_parser = quote! {
              if value_source.is_default() {
                return Ok(#doc_val);
              }
            };

            self.gen_initializer_helper(conf_context_ident, Some(before_value_parser))
        }
    }
}
