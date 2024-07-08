use super::StructItem;
use crate::util::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned, Error, Expr, Field, Ident, LitChar, LitStr};

pub struct ParameterItem {
    field_name: Ident,
    is_optional_type: bool,
    short_switch: Option<LitChar>,
    long_switch: Option<LitStr>,
    env_name: Option<LitStr>,
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
        let is_optional_type = type_is_option(&field.ty);

        let mut result = Self {
            field_name,
            is_optional_type,
            short_switch: None,
            long_switch: None,
            env_name: None,
            default_value: None,
            value_parser: None,
            doc_string: None,
        };

        for attr in &field.attrs {
            maybe_append_doc_string(&mut result.doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") {
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
                    } else if path.is_ident("env") {
                        set_once(
                            &path,
                            &mut result.env_name,
                            parse_optional_value::<LitStr>(meta)?
                                .or(make_env(&result.field_name, path.span())),
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
                    } else {
                        Err(meta.error("unrecognized conf parameter option"))
                    }
                })?;
            }
        }

        if !result.is_optional_type
            && result.short_switch.is_none()
            && result.long_switch.is_none()
            && result.env_name.is_none()
            && result.default_value.is_none()
        {
            return Err(Error::new(field.span(), "There is no way for the user to give this parameter a value. Trying using #[conf(short)], #[conf(long)], or #[conf(env)] to specify a switch or an env associated to this value, or specify a default value."));
        }

        Ok(result)
    }

    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        let is_required = !self.is_optional_type && self.default_value.is_none();
        let id = self.field_name.to_string();
        let description = quote_opt_into(&self.doc_string);
        let short_form = quote_opt(&self.short_switch);
        let long_form = quote_opt_into(&self.long_switch);
        let env_form = quote_opt_into(&self.env_name);
        let default_value = quote_opt_into(&self.default_value);

        Ok(quote! {
            #program_options_ident.push(conf::ProgramOption {
                id: #id.into(),
                parse_type: conf::ParseType::Parameter,
                description: #description,
                short_form: #short_form,
                long_form: #long_form,
                env_form: #env_form,
                default_value: #default_value,
                is_required: #is_required,
            });
        })
    }

    pub fn gen_initializer(&self, conf_context_ident: &Ident) -> Result<TokenStream, syn::Error> {
        let field_name = &self.field_name;
        let id = self.field_name.to_string();

        let short_form = quote_opt(&self.short_switch);
        let long_form = quote_opt(&self.long_switch);
        let env_form = quote_opt(&self.env_name);

        // Value parser is FromStr::from_str if not specified
        let value_parser: Expr = self
            .value_parser
            .clone()
            .unwrap_or_else(|| parse_quote! { std::str::FromStr::from_str });

        // Code gen is slightly different if the field type is Optional<T>
        // The part around value parser needs to be very simple if we want type inference to work
        if !self.is_optional_type {
            Ok(quote! {
                #field_name: {
                  let (value_source, got_str): (conf::ValueSource, &str) = #conf_context_ident.get_string_opt(#id)?.ok_or(conf::InnerError::MissingRequiredParameter(#short_form, #long_form, #env_form))?;
                  match #value_parser(got_str) {
                    Ok(t) => t,
                    Err(err) => return Err(conf::InnerError::InvalidParameterValue(value_source, err.to_string()).into()),
                  }
                },
            })
        } else {
            Ok(quote! {
                #field_name: {
                  let maybe_str: Option<(conf::ValueSource, &str)> = #conf_context_ident.get_string_opt(#id)?;
                  match maybe_str {
                    Some((value_source, got_str)) => {
                      match #value_parser(got_str) {
                        Ok(t) => Some(t),
                        Err(err) => return Err(conf::InnerError::InvalidParameterValue(value_source, err.to_string()).into()),
                      }
                    },
                    None => None,
                  }
                },
            })
        }
    }
}
