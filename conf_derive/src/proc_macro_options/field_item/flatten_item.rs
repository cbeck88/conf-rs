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
    is_optional_type: Option<Type>,
    long_prefix: Option<LitStr>,
    env_prefix: Option<LitStr>,
    description_prefix: Option<String>,
    skip_short: Option<LitCharArray>,
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
        let is_optional_type = type_is_option(&field.ty)?;

        let mut result = Self {
            field_name,
            field_type,
            is_optional_type,
            long_prefix: None,
            env_prefix: None,
            description_prefix: None,
            skip_short: None,
        };

        // These two variables are used to set description_prefix at the end.
        let mut doc_string: Option<String> = None;
        // If help_prefix is set, this is Some
        // If help_prefix sets an explicit value, this is Some(Some(...))
        let mut help_prefix: Option<Option<LitStr>> = None;

        for attr in &field.attrs {
            maybe_append_doc_string(&mut doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
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
                    } else if path.is_ident("skip_short") {
                        set_once(
                            &path,
                            &mut result.skip_short,
                            Some(parse_required_value::<LitCharArray>(meta)?),
                        )
                    } else {
                        Err(meta.error("unrecognized conf flatten option"))
                    }
                })?;
            }
        }

        // If help prefix was not requested, then doc_string should be ignored. If help_prefix was
        // explicitly assigned, then doc_string is shadowed. unwrap_or_default is used to
        // flatten the two levels of Option.
        result.description_prefix = help_prefix
            .map(|inner| inner.as_ref().map(LitStr::value).or(doc_string))
            .unwrap_or_default();

        Ok(result)
    }

    pub fn get_field_name(&self) -> &Ident {
        &self.field_name
    }

    fn get_id_prefix(&self) -> String {
        self.field_name.to_string() + "."
    }

    pub fn get_field_type(&self) -> Type {
        self.field_type.clone()
    }

    // Body of a routine which extends #program_options_ident to hold any program options associated
    // to this field
    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        // Generated code gets all program options for the struct we are flattening, then calls
        // flatten on each one and adds all that to program_options_ident.
        let field_name = self.field_name.to_string();
        let field_type = &self.field_type;
        let id_prefix = self.get_id_prefix();

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
        let skip_short = self.skip_short.as_ref().map(|array| &array.elements);
        let skip_short_len = self
            .skip_short
            .as_ref()
            .map(|array| array.elements.len())
            .unwrap_or(0);

        // Common modifications we have to make to program options whether the flatten is optional
        // or required
        let common_program_option_modifications = quote! {
            .apply_flatten_prefixes(#id_prefix, #long_prefix, #env_prefix, #description_prefix)
            .skip_short_forms(&[#skip_short], &mut was_skipped[..])
        };

        Ok(if let Some(inner_type) = self.is_optional_type.as_ref() {
            // This is flatten-optional. We have to request inner-type program options,
            // and do the same things to them, except also call make_optional() on them at the end.
            quote! {
              let mut was_skipped = [false; #skip_short_len];
              #program_options_ident.extend(
                #inner_type::get_program_options()?.iter().cloned().map(
                  |program_option|
                      program_option
                          #common_program_option_modifications
                          .make_optional()
                )
              );
              if !was_skipped.iter().all(|x| *x) {
                let not_skipped: Vec<char> = [#skip_short].into_iter().zip(was_skipped.into_iter()).filter_map(
                    |(short_form, was_skipped)| if was_skipped { None } else { Some(short_form) }
                ).collect();
                return Err(::conf::Error::skip_short_not_found(not_skipped, #field_name, <#inner_type as ::conf::Conf>::get_name()));
              }
            }
        } else {
            // This is a regular flatten
            quote! {
              let mut was_skipped = [false; #skip_short_len];
              #program_options_ident.extend(
                #field_type::get_program_options()?.iter().cloned().map(
                  |program_option|
                      program_option
                          #common_program_option_modifications
                )
              );
              if !was_skipped.iter().all(|x| *x) {
                let not_skipped: Vec<char> = [#skip_short].into_iter().zip(was_skipped.into_iter()).filter_map(
                    |(short_form, was_skipped)| if was_skipped { None } else { Some(short_form) }
                ).collect();
                return Err(::conf::Error::skip_short_not_found(not_skipped, #field_name, <#field_type as ::conf::Conf>::get_name()));
              }
            }
        })
    }

    // Body of a function taking a &ConfContext returning Result<#field_type,
    // Vec<::conf::InnerError>>
    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        let field_type = &self.field_type;

        let id_prefix = self.get_id_prefix();

        if let Some(inner_type) = self.is_optional_type.as_ref() {
            // This is flatten-optional
            Ok((
                quote! {
                    Ok(if let Some(option_appeared_result) = <#inner_type as ::conf::Conf>::any_program_options_appeared(&#conf_context_ident.for_flattened(#id_prefix)).map_err(|err| vec![err])? {
                        let #conf_context_ident = #conf_context_ident.for_flattened_optional(#id_prefix, <#inner_type as ::conf::Conf>::get_name(), option_appeared_result);
                        Some(<#inner_type as ::conf::Conf>::from_conf_context(#conf_context_ident)?)
                    } else {
                        None
                    })
                },
                true,
            ))
        } else {
            // Non-optional flatten
            Ok((
                quote! {
                    let #conf_context_ident = #conf_context_ident.for_flattened(#id_prefix);
                    <#field_type as ::conf::Conf>::from_conf_context(#conf_context_ident)
                },
                true,
            ))
        }
    }

    // Returns an expression which calls any_program_options_appeared with given conf context.
    // This is used to get errors for one_of constraint failures.
    pub fn any_program_options_appeared_expr(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        let field_type = &self.field_type;
        let id_prefix = self.get_id_prefix();

        if let Some(inner_type) = self.is_optional_type.as_ref() {
            Ok(
                quote! { <#inner_type as ::conf::Conf>::any_program_options_appeared(& #conf_context_ident .for_flattened(#id_prefix)) },
            )
        } else {
            Ok(
                quote! { <#field_type as ::conf::Conf>::any_program_options_appeared(& #conf_context_ident .for_flattened(#id_prefix)) },
            )
        }
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
