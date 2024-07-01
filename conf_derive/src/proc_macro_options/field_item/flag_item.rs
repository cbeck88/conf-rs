use super::StructItem;
use crate::util::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Error, Field, Ident, LitChar, LitStr};

pub struct FlagItem {
    field_name: Ident,
    short_switch: Option<LitChar>,
    long_switch: Option<LitStr>,
    env_name: Option<LitStr>,
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
            env_name: None,
            doc_string: None,
        };

        for attr in &field.attrs {
            maybe_append_doc_string(&mut result.doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") {
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
                    } else if path.is_ident("env") {
                        set_once(
                            &path,
                            &mut result.env_name,
                            parse_optional_value::<LitStr>(meta)?
                                .or(make_env(&result.field_name, path.span())),
                        )
                    } else {
                        Err(meta.error("unrecognized conf flag option"))
                    }
                })?;
            }
        }

        Ok(result)
    }

    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        let description = quote_opt_string(&self.doc_string);
        let short_form = quote_opt_char_string(&self.short_switch);
        let long_form = quote_opt_string(&self.long_switch);
        let env_form = quote_opt_string(&self.env_name);

        Ok(quote! {
            #program_options_ident.push(conf::ProgramOption {
                parse_type: conf::ParseType::Flag,
                description: #description,
                short_form: #short_form,
                long_form: #long_form,
                env_form: #env_form,
                default_value: None,
                is_required: false,
            });
        })
    }

    pub fn gen_initializer(&self, conf_context_ident: &Ident) -> Result<TokenStream, Error> {
        let field_name = &self.field_name;

        let short_form = quote_opt_char(&self.short_switch);
        let long_form = quote_opt(&self.long_switch);
        let env_form = quote_opt(&self.env_name);

        Ok(quote! {
            #field_name: #conf_context_ident.get_boolean_opt(#short_form, #long_form, #env_form)?,
        })
    }
}
