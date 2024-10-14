use super::StructItem;
use crate::util::*;
use heck::{ToKebabCase, ToShoutySnakeCase};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::fmt::Display;
use syn::{meta::ParseNestedMeta, spanned::Spanned, token, Error, Field, Ident, LitStr, Type};

/// #[conf(serde(...))] options listed on a field of Flatten kind
pub struct FlattenSerdeItem {
    pub rename: Option<LitStr>,
    pub skip: bool,
    span: Span,
}

impl FlattenSerdeItem {
    pub fn new(meta: ParseNestedMeta<'_>) -> Result<Self, Error> {
        let mut result = Self {
            rename: None,
            skip: false,
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
                } else {
                    Err(meta.error("unrecognized conf(serde) option"))
                }
            })?;
        }

        Ok(result)
    }
}

impl GetSpan for FlattenSerdeItem {
    fn get_span(&self) -> Span {
        self.span
    }
}

/// Proc macro annotations parsed from a field of Flatten kind
pub struct FlattenItem {
    field_name: Ident,
    field_type: Type,
    is_optional_type: Option<Type>,
    long_prefix: Option<LitStr>,
    env_prefix: Option<LitStr>,
    description_prefix: Option<String>,
    skip_short: Option<LitCharArray>,
    serde: Option<FlattenSerdeItem>,
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
            serde: None,
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
                    } else if path.is_ident("serde") {
                        set_once(&path, &mut result.serde, Some(FlattenSerdeItem::new(meta)?))
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

    fn get_serde_name(&self) -> LitStr {
        self.serde
            .as_ref()
            .and_then(|serde| serde.rename.clone())
            .unwrap_or_else(|| LitStr::new(&self.field_name.to_string(), self.field_name.span()))
    }

    pub fn get_serde_skip(&self) -> bool {
        self.serde.as_ref().map(|serde| serde.skip).unwrap_or(false)
    }

    // Body of a routine which extends #program_options_ident to hold any program options associated
    // to this field.
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

        // Identifier for was_skipped variable, used with skip_short_forms sanity checks.
        // This is an array of bools, one for each skip-short parameter.
        let was_skipped_ident = Ident::new("__was_skipped__", Span::call_site());

        // Common modifications we have to make to program options whether the flatten is optional
        // or required
        let common_program_option_modifications = quote! {
          .apply_flatten_prefixes(#id_prefix, #long_prefix, #env_prefix, #description_prefix)
          .skip_short_forms(&[#skip_short], &mut #was_skipped_ident[..])
        };

        // When using flatten optional, we have to make all program options optional
        // before passing them on to lower layers. If not, there are no additional mods needed.
        let modify_program_option = if self.is_optional_type.is_some() {
            quote! {
              #common_program_option_modifications
              .make_optional()
            }
        } else {
            common_program_option_modifications
        };

        // For flatten-optional with Option<T>, inner_type is T and implements Conf.
        // For regular flaten, inner_type is simply the field type and implements Conf.
        let inner_type = self.is_optional_type.as_ref().unwrap_or(field_type);

        // The initializer simply gets all program options, modifies as needed,
        // and then checks for a skip-short error.
        let push_expr = quote! {
          let mut #was_skipped_ident = [false; #skip_short_len];
          #program_options_ident.extend(
            <#inner_type as ::conf::Conf>::get_program_options()?.iter().cloned().map(
              |program_option|
                program_option
                  #modify_program_option
            )
          );
          if #was_skipped_ident.iter().any(|x| !x) {
            let not_skipped: Vec<char> =
              [#skip_short]
                .into_iter()
                .zip(#was_skipped_ident.into_iter())
                .filter_map(
                  |(short_form, was_skipped)| if was_skipped { None } else { Some(short_form) }
                ).collect();
            return Err(
              ::conf::Error::skip_short_not_found(
                not_skipped,
                #field_name,
                <#inner_type as ::conf::Conf>::get_name()
              )
            );
          }
        };

        Ok(push_expr)
    }

    // Flatten fields don't add subcommands to the conf structure, because we don't support that
    // right now. But we need to catch it and flag an error.
    pub fn gen_push_subcommands(
        &self,
        _subcommands_ident: &Ident,
        parsed_env_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        let inner_type: &Type = self.is_optional_type.as_ref().unwrap_or(&self.field_type);
        let type_name = quote! { inner_type }.to_string();
        let panic_message = format!(
          "It is not supported to declare subcommands in a flattened structure '{type_name}', only \
          at top level. (Needs design work around prefixing.)");

        Ok(quote! {
            if !<#inner_type as conf::Conf>::get_subcommands(#parsed_env_ident)?.is_empty() {
              panic!(#panic_message);
            }
        })
    }

    // Body of a function taking a &ConfContext returning
    // Result<#field_type, Vec<::conf::InnerError>>
    //
    // Arguments:
    // * conf_context_ident is the identifier of a &ConfContext variable in scope
    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        let field_type = &self.field_type;

        let id_prefix = self.get_id_prefix();

        let initializer = if let Some(inner_type) = self.is_optional_type.as_ref() {
            // This is flatten-optional
            quote! {
              let option_appeared_result =
                <#inner_type as ::conf::Conf>::any_program_options_appeared(
                  &#conf_context_ident.for_flattened(#id_prefix)
                ).map_err(|err| vec![err])?;
              Ok(if let Some(option_appeared) = option_appeared_result {
                let #conf_context_ident = #conf_context_ident.for_flattened_optional(
                  #id_prefix,
                  <#inner_type as ::conf::Conf>::get_name(),
                  option_appeared
                );
                Some(<#inner_type as ::conf::Conf>::from_conf_context(#conf_context_ident)?)
              } else {
                None
              })
            }
        } else {
            // Non-optional flatten
            quote! {
              let #conf_context_ident = #conf_context_ident.for_flattened(#id_prefix);
              <#field_type as ::conf::Conf>::from_conf_context(#conf_context_ident)
            }
        };
        Ok((initializer, true))
    }

    // Returns an expression which calls any_program_options_appeared with given conf context.
    // This is used to get errors for one_of constraint failures.
    //
    // Arguments:
    // * conf_context_ident is the identifier of a ConfContext variable that is in scope that we
    //   won't consume.
    pub fn any_program_options_appeared_expr(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        let field_type = &self.field_type;
        let id_prefix = self.get_id_prefix();

        let inner_type = self.is_optional_type.as_ref().unwrap_or(field_type);

        Ok(quote! {
          <#inner_type as ::conf::Conf>::any_program_options_appeared(
            & #conf_context_ident .for_flattened(#id_prefix)
          )
        })
    }

    // This is used by ConfSerde
    //
    // When walking a map access, we match on the key, and if we get the key for this field,
    // try to deserialize using DeserializeSeed.
    //
    // We have to use DeserializeSeed with the inner-type if this is flatten-optional, because
    // DeserializeSeed isn't implemented for type on Option<S>.
    //
    // We don't use the "any program options appeared" stuff here because, the key for the flattened
    // structure appeared, so that turns the group on. If that key doesn't appear when we do the
    // serde walk, then later we will try to initialize the group normally.
    //
    // Arguments:
    // * ctxt: identifier of a &ConfSerdeContext in scope
    // * map_access: identifier of a MapAccess in scope
    // * map_access_type: identifier of the MapAccess type in this scope
    // * errors_ident: identifier of a mut Vec<InnerError> errors buffer to which we can push
    pub fn gen_serde_match_arm(
        &self,
        ctxt: &Ident,
        map_access: &Ident,
        map_access_type: &Ident,
        errors_ident: &Ident,
    ) -> Result<(TokenStream, Vec<LitStr>), Error> {
        let field_name = &self.field_name;
        let field_name_str = field_name.to_string();
        let field_type = &self.field_type;
        let serde_name_str = self.get_serde_name();
        let id_prefix = self.get_id_prefix();

        // It's necessary to do special handling for optional flattened structs here,
        // because ConfSerdeContext is only implemented on the inner one.
        //
        // We also don't *have* to do any of the "any program option appeared" stuff here,
        // because we only reach this line if serde provided a value for this struct,
        // and that should mean that the optional group is enabled, since serde mentioned it.
        //
        // Note: We may want to consider an attribute that changes this behavior, so
        // that if the group is not mentioned in args or env then it gets skipped even if
        // serde has some values.
        //
        // As it stands, if serde doesn't mention it, then the `Conf::any_program_option_appeared`
        // stuff runs in the deserializer_finalizer routine when we call Conf::from_conf_context
        // And if serde does mention it, then we have to try to deserialize regardless of what
        // `Conf::any_program_option_appeared` says.
        let inner_type = self.is_optional_type.as_ref().unwrap_or(field_type);

        let val_expr = if self.is_optional_type.is_some() {
            quote! { Some(__val__) }
        } else {
            quote! { __val__ }
        };

        // Note: If next_value_seed returns Err rather than Ok(Err), then I believe it means
        // that our DeserializeSeed implementation never ran, since it never does that.
        // But it's possible that the MapAccess will fail before even getting to that point,
        // and then it could return a singular D::Error. So we should not unwrap such errors.
        let match_arm = quote! {
          #serde_name_str => {
            if #field_name.is_some() {
              #errors_ident.push(
                InnerError::serde(
                  #ctxt.document_name,
                  #field_name_str,
                  #map_access_type::Error::duplicate_field(#serde_name_str)
                )
              );
            } else {
              let __seed__ = <#inner_type as ConfSerde>::Seed::from(
                #ctxt.for_flattened(#id_prefix)
              );
              #field_name = Some(match #map_access.next_value_seed(__seed__) {
                Ok(Ok(__val__)) => {
                  Some(#val_expr)
                }
                Ok(Err(__errs__)) => {
                  #errors_ident.extend(__errs__);
                  None
                }
                Err(__err__) => {
                  #errors_ident.push(
                    InnerError::serde(
                      #ctxt.document_name,
                      #field_name_str,
                      __err__
                    )
                  );
                  None
                }
              });
            }
          },
        };
        Ok((match_arm, vec![serde_name_str]))
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
