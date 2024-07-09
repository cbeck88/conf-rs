use super::StructItem;
use crate::util::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned, Error, Field, Ident, LitChar, LitStr, Type};

pub struct FlagItem {
    field_name: Ident,
    short_switch: Option<LitChar>,
    long_switch: Option<LitStr>,
    aliases: Option<LitStrArray>,
    env_name: Option<LitStr>,
    env_aliases: Option<LitStrArray>,
    doc_string: Option<String>,
}

impl FlagItem {
    pub fn new(field: &Field, _struct_item: &StructItem) -> Result<Self, Error> {
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "missing identifier"))?;

        let mut result = Self {
            field_name,
            short_switch: None,
            long_switch: None,
            aliases: None,
            env_name: None,
            env_aliases: None,
            doc_string: None,
        };

        for attr in &field.attrs {
            maybe_append_doc_string(&mut result.doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("flag") {
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
                    } else {
                        Err(meta.error("unrecognized conf flag option"))
                    }
                })?;
            }
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
        parse_quote! { bool }
    }

    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, Error> {
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

        Ok(quote! {
            #program_options_ident.push(::conf::ProgramOption {
                id: #id.into(),
                parse_type: ::conf::ParseType::Flag,
                description: #description,
                short_form: #short_form,
                long_form: #long_form,
                aliases: vec![#aliases],
                env_form: #env_form,
                env_aliases: vec![#env_aliases],
                default_value: None,
                is_required: false,
                allow_hyphen_values: false,
                secret: Some(false),
            });
        })
    }

    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), Error> {
        let id = self.field_name.to_string();

        Ok((
            quote! {
                let (_src, val) = #conf_context_ident.get_boolean_opt(#id)?;
                Ok(val)
            },
            false,
        ))
    }
}
