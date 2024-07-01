use super::StructItem;
use crate::util::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned, Error, Expr, Field, Ident, LitChar, LitStr, Type};

pub struct RepeatItem {
    field_name: Ident,
    field_type: Type, // This is needed to help with type inference in code gen
    long_switch: Option<LitStr>,
    env_name: Option<LitStr>,
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

        let mut result = Self {
            field_name,
            field_type,
            long_switch: None,
            env_name: None,
            value_parser: None,
            env_delimiter: None,
            no_env_delimiter: false,
            description: None,
        };

        for attr in &field.attrs {
            maybe_append_doc_string(&mut result.description, &attr.meta)?;
            if attr.path().is_ident("conf") {
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
                    } else if path.is_ident("env") {
                        set_once(
                            &path,
                            &mut result.env_name,
                            parse_optional_value::<LitStr>(meta)?
                                .or(make_env(&result.field_name, path.span())),
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

        Ok(result)
    }

    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        let description = quote_opt_string(&self.description);
        let long_form = quote_opt_string(&self.long_switch);
        let env_form = quote_opt_string(&self.env_name);

        Ok(quote! {
            #program_options_ident.push(conf::ProgramOption {
                parse_type: conf::ParseType::Repeat,
                description: #description,
                short_form: None,
                long_form: #long_form,
                env_form: #env_form,
                default_value: None,
                is_required: false,
            });
        })
    }

    pub fn gen_initializer(&self, conf_context_ident: &Ident) -> Result<TokenStream, syn::Error> {
        // Generated code is like:
        //
        // #field_name: conf_context.get_repeat_opt(...)?.iter().map(value_parser).collect::<Result<Vec<_>, _>().map_err(Error::InvalidParameterValue)?,

        let field_name = &self.field_name;
        let field_type = &self.field_type;

        let long_form = quote_opt(&self.long_switch);
        // Argument to conf_context.get_repeat_opt includes both the env form and any configured delimiter to split on
        let env_form = if let Some(name) = self.env_name.as_ref() {
            if self.no_env_delimiter {
                quote! {
                    Some((#name, None))
                }
            } else {
                // delimiter defaults to char if not specified
                let delim = self
                    .env_delimiter
                    .clone()
                    .unwrap_or_else(|| LitChar::new(',', name.span()));
                quote! {
                    Some((#name, Some(#delim)))
                }
            }
        } else {
            quote! { None }
        };
        // Value parser is FromStr::from_str if not specified
        let value_parser = self
            .value_parser
            .clone()
            .unwrap_or_else(|| parse_quote! { std::str::FromStr::from_str });

        // Note: We can't use rust into_iter, collect, map_err because sometimes it messes with type inference around the value parser
        // Note: The line `let mut result: #field_type = Default::default();` is expected to be default initializing a Vec.
        // If it fails because the user put another funky type there, imo this should not breally e supported.
        // It's more compelling to make the value_parser option easier to use (easier type inference) than to support user-defined containers here,
        // and try to use `.collect` etc. directly into their container. The user's code can do .iter().collect() after our code runs if they want.
        // Or, they can use a custom type that has `Default` and `.push()`.
        Ok(quote! {
            #field_name: {
                let (value_source, strs): (conf::ValueSource, Vec<&str>) = #conf_context_ident.get_repeat_opt(#long_form, #env_form)?;
                let mut result: #field_type = Default::default();
                result.reserve(strs.len());
                for arg in strs {
                    match #value_parser(arg) {
                        Ok(val) => result.push(val),
                        Err(err) => return Err(conf::InnerError::InvalidParameterValue(value_source, err.to_string()).into()),
                    }
                }
                result
            },
        })
    }
}
