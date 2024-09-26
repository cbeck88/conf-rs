use super::StructItem;
use crate::util::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_quote, spanned::Spanned, Error, Expr, Field, Ident, LitBool, LitChar, LitStr, Type,
};

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
    doc_string: Option<String>,
}

impl ParameterItem {
    pub fn new(field: &Field, _struct_item: &StructItem) -> Result<Self, Error> {
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "missing identifier"))?;
        let field_type = field.ty.clone();
        let is_optional_type = type_is_option(&field.ty)?;
        let allow_hyphen_values = type_is_signed_number(&field.ty); // signed numbers often start with hyphens

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
        {
            return Err(Error::new(field.span(), "There is no way for the user to give this parameter a value. Trying using #[conf(short)], #[conf(long)], or #[conf(env)] to specify a switch or an env associated to this value, or specify a default value."));
        }

        if result.long_switch.is_none()
            && !result
                .aliases
                .as_ref()
                .map(LitStrArray::is_empty)
                .unwrap_or(true)
        {
            return Err(Error::new(field.span(), "Setting aliases without setting a long-switch is an error, make one of the aliases the primary switch name."));
        }

        if result.env_name.is_none()
            && !result
                .env_aliases
                .as_ref()
                .map(LitStrArray::is_empty)
                .unwrap_or(true)
        {
            return Err(Error::new(field.span(), "Setting env_aliases without setting an env is an error, make one of the aliases the primary env."));
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

    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        let field_type = &self.field_type;
        let id = self.field_name.to_string();

        // Value parser is FromStr::from_str if not specified
        let value_parser: Expr = self
            .value_parser
            .clone()
            .unwrap_or_else(|| parse_quote! { ::core::str::FromStr::from_str });

        // Code gen is slightly different if the field type is Optional<T>
        // The part around value parser needs to be very simple if we want type inference to work
        // We also stick the user-provided expression inside a function to prevent it from mutating
        // anything in the surrounding scope.
        // But we are reading conf_context_ident from our caller's scope, outside of the
        // user-provided expression
        Ok((
            if let Some(inner_type) = self.is_optional_type.as_ref() {
                quote! {
                    {
                      fn value_parser(__arg__: &str) -> Result<#inner_type, impl ::core::fmt::Display> {
                        #value_parser(__arg__)
                      }

                      let (maybe_val, opt): (Option<_>, &conf::ProgramOption) = #conf_context_ident.get_string_opt(#id)?;
                      match maybe_val {
                        Some((value_source, val_str)) => {
                          match value_parser(val_str) {
                            Ok(t) => Ok(Some(t)),
                            Err(err) => Err(::conf::InnerError::invalid_value(value_source, val_str, opt, err)),
                          }
                        },
                        None => Ok(None),
                      }
                    }
                }
            } else {
                quote! {
                    {
                      fn value_parser(__arg__: &str) -> Result<#field_type, impl ::core::fmt::Display> {
                        #value_parser(__arg__)
                      }

                      let (maybe_val, opt): (Option<_>, &::conf::ProgramOption) = #conf_context_ident.get_string_opt(#id)?;
                      let (value_source, val_str): (::conf::ConfValueSource<&str>, &str) = maybe_val.ok_or_else(|| #conf_context_ident.missing_required_parameter_error(opt))?;
                      match value_parser(val_str) {
                        Ok(t) => Ok(t),
                        Err(err) => Err(::conf::InnerError::invalid_value(value_source, val_str, opt, err)),
                      }
                    }
                }
            },
            false,
        ))
    }
}
