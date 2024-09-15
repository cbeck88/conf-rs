use super::StructItem;
use crate::util::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_quote, spanned::Spanned, Error, Expr, Field, Ident, LitBool, LitChar, LitStr, Type,
};

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
            #program_options_ident.push(conf::ProgramOption {
                id: #id.into(),
                parse_type: conf::ParseType::Repeat,
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

    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        let field_type = &self.field_type;
        let id = self.field_name.to_string();

        let delimiter = quote_opt(&if self.no_env_delimiter {
            None
        } else {
            Some(
                self.env_delimiter
                    .clone()
                    .unwrap_or_else(|| LitChar::new(',', self.field_name.span())),
            )
        });

        // Value parser is FromStr::from_str if not specified
        let value_parser = self
            .value_parser
            .clone()
            .unwrap_or_else(|| parse_quote! { std::str::FromStr::from_str });

        // Note: We can't use rust into_iter, collect, map_err because sometimes it messes with type
        // inference around the value parser Note: The line `let mut result: #field_type =
        // Default::default();` is expected to be default initializing a Vec. If it fails
        // because the user put another funky type there, imo this should not really be supported.
        // It's more compelling to make the value_parser option easier to use (easier type
        // inference) than to support user-defined containers here, and try to use
        // `.collect` etc. directly into their container. The user's code can do .iter().collect()
        // after our code runs if they want.
        Ok((
            quote! {
               || -> Result<#field_type, Vec<conf::InnerError>> {
                    let (value_source, strs, opt): (conf::ConfValueSource<&str>, Vec<&str>, &conf::ProgramOption) = #conf_context_ident.get_repeat_opt(#id, #delimiter).map_err(|err| vec![err])?;
                    let mut result: #field_type = Default::default();
                    let mut errors = Vec::<conf::InnerError>::new();
                    result.reserve(strs.len());
                    for val_str in strs {
                        match #value_parser(val_str) {
                            Ok(val) => result.push(val),
                            Err(err) => errors.push(conf::InnerError::invalid_value(value_source.clone(), val_str, opt, err.to_string()).into()),
                        }
                    }
                    if errors.is_empty() {
                        Ok(result)
                    } else {
                        Err(errors)
                    }
                }()
            },
            true,
        ))
    }
}
