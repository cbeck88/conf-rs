use super::StructItem;
use crate::util::*;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    meta::ParseNestedMeta, parse_quote, spanned::Spanned, token, Error, Expr, Field, Ident,
    LitBool, LitChar, LitStr, Type,
};

/// #[conf(serde(...))] options listed on a parameter
pub struct ParameterSerdeItem {
    pub rename: Option<LitStr>,
    pub skip: bool,
    pub use_value_parser: bool,
    span: Span,
}

impl ParameterSerdeItem {
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

impl GetSpan for ParameterSerdeItem {
    fn get_span(&self) -> Span {
        self.span
    }
}

/// Proc macro annotations parsed from a field of Parameter kind
pub struct ParameterItem {
    field_name: Ident,
    field_type: Type,
    is_optional_type: Option<Type>,
    allow_hyphen_values: bool,
    secret: Option<LitBool>,
    short_switch: Option<LitChar>,
    long_switch: Option<LitStr>,
    aliases: Option<LitStrArray>,
    env_name: Option<LitStr>,
    env_aliases: Option<LitStrArray>,
    default_value: Option<LitStr>,
    value_parser: Option<Expr>,
    serde: Option<ParameterSerdeItem>,
    doc_string: Option<String>,
}

impl ParameterItem {
    pub fn new(field: &Field, struct_item: &StructItem) -> Result<Self, Error> {
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "missing identifier"))?;
        let field_type = field.ty.clone();
        let is_optional_type = type_is_option(&field.ty)?;
        // signed numbers often start with hyphens
        let allow_hyphen_values = type_is_signed_number(&field.ty);

        let mut result = Self {
            field_name,
            field_type,
            is_optional_type,
            allow_hyphen_values,
            secret: None,
            short_switch: None,
            long_switch: None,
            aliases: None,
            env_name: None,
            env_aliases: None,
            default_value: None,
            value_parser: None,
            serde: None,
            doc_string: None,
        };

        for attr in &field.attrs {
            maybe_append_doc_string(&mut result.doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("parameter") {
                        Ok(())
                    } else if path.is_ident("short") {
                        set_once(
                            &path,
                            &mut result.short_switch,
                            parse_optional_value::<LitChar>(meta)?
                                .or(make_short(&result.field_name, path.span())),
                        )
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
                            &mut result.env_aliases,
                            Some(parse_required_value::<LitStrArray>(meta)?),
                        )
                    } else if path.is_ident("default_value") {
                        let val = meta.value()?.parse::<LitStr>()?;
                        set_once(&path, &mut result.default_value, Some(val))
                    } else if path.is_ident("value_parser") {
                        set_once(
                            &path,
                            &mut result.value_parser,
                            Some(parse_required_value::<Expr>(meta)?),
                        )
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
                        set_once(
                            &path,
                            &mut result.serde,
                            Some(ParameterSerdeItem::new(meta)?),
                        )
                    } else {
                        Err(meta.error("unrecognized conf parameter option"))
                    }
                })?;
            }
        }

        if result.is_optional_type.is_none()
            && result.short_switch.is_none()
            && result.long_switch.is_none()
            && result.env_name.is_none()
            && result.default_value.is_none()
            && struct_item.serde.is_none()
        {
            return Err(Error::new(
                field.span(),
                "There is no way for the user to give this parameter a value. \
                Trying using #[arg(short)], #[arg(long)], or #[arg(env)] to specify a switch \
                or an env associated to this value, or specify a default value.",
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

    pub fn get_default_value(&self) -> Option<&LitStr> {
        self.default_value.as_ref()
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
            parse_quote! { ::std::string::String }
        } else {
            self.field_type.clone()
        }
    }

    pub fn get_serde_skip(&self) -> bool {
        self.serde.as_ref().map(|serde| serde.skip).unwrap_or(false)
    }

    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        let is_required = self.is_optional_type.is_none() && self.default_value.is_none();
        let id = self.field_name.to_string();
        let description = quote_opt_into(&self.doc_string);
        let short_form = quote_opt(&self.short_switch);
        let long_form = quote_opt_into(&self.long_switch);
        let aliases = self.aliases.as_ref().map(LitStrArray::quote_elements_into);
        let env_form = quote_opt_into(&self.env_name);
        let env_aliases = self
            .env_aliases
            .as_ref()
            .map(LitStrArray::quote_elements_into);
        let default_value = quote_opt_into(&self.default_value);
        let allow_hyphen_values = self.allow_hyphen_values;
        let secret = quote_opt(&self.secret);

        Ok(quote! {
            #program_options_ident.push(::conf::ProgramOption {
                id: #id.into(),
                parse_type: ::conf::ParseType::Parameter,
                description: #description,
                short_form: #short_form,
                long_form: #long_form,
                aliases: vec![#aliases],
                env_form: #env_form,
                env_aliases: vec![#env_aliases],
                default_value: #default_value,
                is_required: #is_required,
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

    fn get_value_parser(&self) -> Expr {
        // Value parser is FromStr::from_str if not specified
        self.value_parser
            .clone()
            .unwrap_or_else(|| parse_quote! { std::str::FromStr::from_str })
    }

    fn gen_initializer_helper(
        &self,
        conf_context_ident: &Ident,
        if_no_conf_context_val: Option<TokenStream>,
        before_value_parser: Option<TokenStream>,
    ) -> Result<(TokenStream, bool), syn::Error> {
        let field_type = &self.field_type;
        let id = self.field_name.to_string();

        let value_parser = self.get_value_parser();

        // Code gen is slightly different if the field type is Option<T>
        // Inner_type is T in that case, or just field_type otherwise.
        // Value parser will produce inner_type.
        let inner_type = self.is_optional_type.as_ref().unwrap_or(field_type);

        // If the conf context doesn't find a value, its an error if this field is required,
        // and Ok(None) if this field is optional.
        // This gets overrided when serde provides a value.
        let if_no_conf_context_val = if_no_conf_context_val.unwrap_or_else(|| {
            if self.is_optional_type.is_some() {
                quote! { return Ok(None); }
            } else {
                quote! { return Err(#conf_context_ident.missing_required_parameter_error(opt)); }
            }
        });

        // Value parser produces #inner_type, so we have to massage a success result to #field_type
        let value_parser_ok_arm = if self.is_optional_type.is_some() {
            quote! { Ok(t) => Ok(Some(t)), }
        } else {
            quote! { Ok(t) => Ok(t), }
        };

        // The part around value parser needs to be very simple if we want type inference to work
        // We also stick the user-provided expression inside a function to prevent it from mutating
        // anything in the surrounding scope.
        // But we are reading conf_context_ident from our caller's scope, outside of the
        // user-provided expression
        let initializer = quote! {
          {
            fn __value_parser__(
              __arg__: &str
            ) -> Result<#inner_type, impl ::core::fmt::Display> {
              #value_parser(__arg__)
            }

            use ::conf::{ConfValueSource, ProgramOption, InnerError};

            let (maybe_val, opt): (Option<_>, &ProgramOption)
              = #conf_context_ident.get_string_opt(#id)?;
            let (value_source, val_str): (ConfValueSource<&str>, &str)
              = if let Some(val) = maybe_val {
                val
              } else {
                #if_no_conf_context_val
              };
            #before_value_parser
            match __value_parser__(val_str) {
              #value_parser_ok_arm
              Err(err) => Err(
                InnerError::invalid_value(
                  value_source,
                  val_str,
                  opt,
                  err
                )
              ),
            }
          }
        };
        Ok((initializer, false))
    }

    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        self.gen_initializer_helper(conf_context_ident, None, None)
    }

    // Gen initializer with a provided document value.
    //
    // Like gen_initializer, but in this case, serde has provided a value for this field.
    // The value is a variable of type #serde_type and the identifier is #doc_val.
    //
    // Here, we should return #doc_val if the basic initializer would have produced no value,
    // or would have produced the default_value string, because the document is higher priority.
    // But the document value should be ignored if args or env is the value source.
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
            // When use_value_parser is true, then #doc_val has type String.
            // To pick this value for the field, we have to set value_source and val_str
            // to indicate that we are selecting the document value.
            let if_no_conf_context_val = quote! {
              (ConfValueSource::Document(#doc_name), #doc_val.as_str())
            };

            // When the value source is a default, but we have a doc val,
            // we should prefer the doc val.
            let before_value_parser = quote! {
              let (value_source, val_str) = if value_source.is_default() {
                #if_no_conf_context_val
              } else {
                (value_source, val_str)
              };
            };

            self.gen_initializer_helper(
                conf_context_ident,
                Some(if_no_conf_context_val),
                Some(before_value_parser),
            )
        } else {
            // When use_value_parser is false, then #doc_val has type #field_type.
            // To pick this value for the field, we just return it.
            let if_no_conf_context_val = quote! {
              return Ok(#doc_val);
            };

            // After we have a conf context value, but before we run the value parser,
            // check if the value that was obtained should be lower priority than the doc val.
            // If so then early return in the same way.
            let before_value_parser = quote! {
              if value_source.is_default() {
                #if_no_conf_context_val
              }
            };
            self.gen_initializer_helper(
                conf_context_ident,
                Some(if_no_conf_context_val),
                Some(before_value_parser),
            )
        }
    }
}
