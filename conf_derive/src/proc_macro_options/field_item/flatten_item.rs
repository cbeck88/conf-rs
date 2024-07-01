use super::StructItem;
use crate::util::*;
use heck::{ToKebabCase, ToShoutySnakeCase};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::fmt::Display;
use syn::{spanned::Spanned, Error, Field, Ident, LitStr, Type};

pub struct FlattenItem {
    field_name: Ident,
    field_type: Type,
    long_prefix: Option<LitStr>,
    env_prefix: Option<LitStr>,
    description_prefix: Option<String>,
}

fn make_long_prefix(ident: &impl Display, span: Span) -> Option<LitStr> {
    let formatted = format!("{}-", ident.to_string().to_kebab_case());
    Some(LitStr::new(&formatted, span))
}

fn make_env_prefix(ident: &impl Display, span: Span) -> Option<LitStr> {
    let formatted = format!("{}_", ident.to_string().to_shouty_snake_case());
    Some(LitStr::new(&formatted, span))
}

impl FlattenItem {
    pub fn new(field: &Field, _struct_item: &StructItem) -> Result<Self, Error> {
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "missing identifier"))?;
        let field_type = field.ty.clone();

        let mut result = Self {
            field_name,
            field_type,
            long_prefix: None,
            env_prefix: None,
            description_prefix: None,
        };

        // These two variables are used to set description_prefix at the end.
        let mut doc_string: Option<String> = None;
        // If help_prefix is set, this is Some
        // If help_prefix sets an explicit value, this is Some(Some(...))
        let mut help_prefix: Option<Option<LitStr>> = None;

        for attr in &field.attrs {
            maybe_append_doc_string(&mut doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("flatten") {
                        Ok(())
                    } else if path.is_ident("long_prefix") {
                        set_once(
                            &path,
                            &mut result.long_prefix,
                            parse_optional_value::<LitStr>(meta)?
                                .or(make_long_prefix(&result.field_name, path.span())),
                        )
                    } else if path.is_ident("env_prefix") {
                        set_once(
                            &path,
                            &mut result.env_prefix,
                            parse_optional_value::<LitStr>(meta)?
                                .or(make_env_prefix(&result.field_name, path.span())),
                        )
                    } else if path.is_ident("help_prefix") {
                        set_once(
                            &path,
                            &mut help_prefix,
                            Some(parse_optional_value::<LitStr>(meta)?),
                        )
                    } else if path.is_ident("prefix") {
                        let (long_prefix, env_prefix) = match parse_optional_value::<LitStr>(meta)?
                        {
                            Some(prefix) => (
                                make_long_prefix(&prefix.value(), path.span()),
                                make_env_prefix(&prefix.value(), path.span()),
                            ),
                            None => (
                                make_long_prefix(&result.field_name, path.span()),
                                make_env_prefix(&result.field_name, path.span()),
                            ),
                        };
                        set_once(&path, &mut result.long_prefix, long_prefix)?;
                        set_once(&path, &mut result.env_prefix, env_prefix)?;
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized conf flatten option"))
                    }
                })?;
            }
        }

        // If help prefix was not requested, then doc_string should be ignored. If help_prefix was explicitly assigned, then doc_string is shadowed.
        // unwrap_or_default is used to flatten the two levels of Option.
        result.description_prefix = help_prefix
            .map(|inner| inner.as_ref().map(LitStr::value).or(doc_string))
            .unwrap_or_default();

        Ok(result)
    }

    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        // Generated code gets all program options for the struct we are flattening, then calls flatten on each one and adds all that to program_options_ident.
        let field_type = &self.field_type;

        let long_prefix = self
            .long_prefix
            .as_ref()
            .map(LitStr::value)
            .unwrap_or_default();
        let env_prefix = self
            .env_prefix
            .as_ref()
            .map(LitStr::value)
            .unwrap_or_default();
        let description_prefix = self.description_prefix.as_deref().unwrap_or_default();

        Ok(quote! {
            #program_options_ident.extend(#field_type::get_program_options()?.1.into_iter().map(|program_option| program_option.flatten(#long_prefix, #env_prefix, #description_prefix)));
        })
    }

    pub fn gen_initializer(&self, conf_context_ident: &Ident) -> Result<TokenStream, syn::Error> {
        // Generated code is like:
        //
        // #field_name: Type::from_conf_context(conf_context.for_flattened(long_prefix, env_prefix))?,

        let field_name = &self.field_name;
        let field_type = &self.field_type;

        let long_prefix = self
            .long_prefix
            .as_ref()
            .map(LitStr::value)
            .unwrap_or_default();
        let env_prefix = self
            .env_prefix
            .as_ref()
            .map(LitStr::value)
            .unwrap_or_default();

        Ok(quote! {
            #field_name: #field_type::from_conf_context(#conf_context_ident.for_flattened(#long_prefix, #env_prefix))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_long_prefix() {
        let result = make_long_prefix(&"my_field", Span::call_site()).unwrap();
        assert_eq!(result.value(), "my-field-");
    }

    #[test]
    fn test_make_env_prefix() {
        let result = make_env_prefix(&"my_field", Span::call_site()).unwrap();
        assert_eq!(result.value(), "MY_FIELD_");
    }
}
