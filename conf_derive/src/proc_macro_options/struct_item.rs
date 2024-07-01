use crate::util::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, LitStr};

/// #[conf(...)] options listed on a struct which has `#[derive(Conf)]`
#[derive(Default)]
pub struct StructItem {
    pub about: Option<LitStr>,
    pub no_help_flag: bool,
    pub env_prefix: Option<LitStr>,
    pub doc_string: Option<String>,
}

impl StructItem {
    /// Parse conf options out of attributes on a struct
    pub fn new(attrs: &[Attribute]) -> Result<Self, syn::Error> {
        let mut result = Self::default();

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
                    } else if path.is_ident("env_prefix") {
                        set_once(
                            &path,
                            &mut result.env_prefix,
                            Some(parse_required_value::<LitStr>(meta)?),
                        )
                    } else {
                        Err(meta.error("unrecognized conf option"))
                    }
                })?;
            }
        }

        Ok(result)
    }

    /// Generate a conf::ParserConfig expression, based on top-level options in this struct
    pub fn gen_parser_config(&self) -> Result<TokenStream, syn::Error> {
        let no_help_flag = self.no_help_flag;
        let about_text = self
            .about
            .as_ref()
            .map(|lit_str| lit_str.value())
            .or(self.doc_string.clone());
        let about = quote_opt_string(&about_text);
        Ok(quote! {
            conf::ParserConfig {
                about: #about,
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
        if let Some(env_prefix) = self.env_prefix.as_ref() {
            Ok(Some(quote! {
                #program_options_ident = #program_options_ident.into_iter().map(|opt| opt.flatten("", #env_prefix, "")).collect();
            }))
        } else {
            Ok(None)
        }
    }

    /// Generate an (optional) ConfContext pre-processing step.
    /// If we have an env_prefix at struct-level, apply it here.
    pub fn gen_pre_process_conf_context(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<Option<TokenStream>, syn::Error> {
        if let Some(env_prefix) = self.env_prefix.as_ref() {
            Ok(Some(quote! {
                let #conf_context_ident = #conf_context_ident.for_flattened("", #env_prefix);
            }))
        } else {
            Ok(None)
        }
    }
}
