use crate::util::*;
use heck::{ToKebabCase, ToSnakeCase};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Error, Fields, FieldsUnnamed, Ident, LitStr, Type, Variant, meta::ParseNestedMeta,
    spanned::Spanned, token,
};

/// #[conf(serde(...))] options listed on a field of Flatten kind
pub struct VariantSerdeItem {
    pub rename: Option<LitStr>,
    pub skip: bool,
    span: Span,
}

impl VariantSerdeItem {
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

impl GetSpan for VariantSerdeItem {
    fn get_span(&self) -> Span {
        self.span
    }
}

/// Proc macro annotations parsed from a variant within a Subcommands enum
pub struct VariantItem {
    variant_name: Ident,
    variant_type: Option<Type>, // None when we have a unit variant, Some otherwise
    is_optional_type: Option<Type>, // Some when we have a single unnamed field which is Option<T>
    command_name: LitStr,
    aliases: Vec<LitStr>,
    serde: Option<VariantSerdeItem>,
    doc_string: Option<String>,
}

impl VariantItem {
    pub fn new(variant: &Variant, _enum_ident: &Ident) -> Result<Self, Error> {
        let variant_name = variant.ident.clone();

        let (variant_type, is_optional_type) = match variant.fields {
            Fields::Unit => (None, None),
            Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => {
                match unnamed.len() {
                    //0 => (None, None),
                    1 => {
                        let field = unnamed.first().unwrap();

                        let variant_type = field.ty.clone();
                        let is_optional_type = type_is_option(&variant_type)?;

                        (Some(variant_type), is_optional_type)
                    }
                    n => {
                        return Err(Error::new(
                            unnamed.span(),
                            format!(
                                "Subcommands variant '{variant_name}' must contain zero or one unnamed fields which implement Conf, found {n}"
                            ),
                        ));
                    }
                }
            }
            Fields::Named(_) => {
                return Err(Error::new(
                    variant.fields.span(),
                    format!(
                        "Subcommands variant '{variant_name}' must contain zero or one unnamed fields which implement Conf, found named fields"
                    ),
                ));
            }
        };

        let mut result = Self {
            command_name: LitStr::new(
                &variant_name.to_string().to_kebab_case(),
                variant_name.span(),
            ),
            variant_name,
            variant_type,
            is_optional_type,
            aliases: Vec::new(),
            serde: None,
            doc_string: None,
        };

        let mut command_name_override: Option<LitStr> = None;

        for attr in &variant.attrs {
            maybe_append_doc_string(&mut result.doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") || attr.path().is_ident("subcommands") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("subcommand") {
                        Ok(())
                    } else if path.is_ident("name") {
                        set_once(
                            &path,
                            &mut command_name_override,
                            Some(parse_required_value::<LitStr>(meta)?),
                        )
                    } else if path.is_ident("alias") {
                        result.aliases.push(parse_required_value::<LitStr>(meta)?);
                        Ok(())
                    } else if path.is_ident("serde") {
                        set_once(&path, &mut result.serde, Some(VariantSerdeItem::new(meta)?))
                    } else {
                        Err(meta.error("unrecognized conf subcommands option"))
                    }
                })?;
            }
        }

        if let Some(command_name) = command_name_override {
            result.command_name = command_name;
        }

        Ok(result)
    }

    pub fn get_name(&self) -> &Ident {
        &self.variant_name
    }

    pub fn get_command_name(&self) -> &LitStr {
        &self.command_name
    }

    pub fn get_all_command_names(&self) -> Vec<&LitStr> {
        std::iter::once(&self.command_name)
            .chain(self.aliases.iter())
            .collect()
    }

    pub fn get_serde_name(&self) -> LitStr {
        self.serde
            .as_ref()
            .and_then(|serde| serde.rename.clone())
            .unwrap_or_else(|| {
                LitStr::new(
                    &self.variant_name.to_string().to_snake_case(),
                    self.variant_name.span(),
                )
            })
    }

    pub fn get_serde_skip(&self) -> bool {
        self.serde.as_ref().map(|serde| serde.skip).unwrap_or(false)
    }

    pub fn gen_from_conf_context_match_arm(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        let name = self.get_name();
        let all_names = self.get_all_command_names();

        if let Some(ty) = self.variant_type.as_ref() {
            Ok(quote! {
                #(#all_names)|* => Ok(Self::#name(<#ty as Conf>::from_conf_context(#conf_context_ident)?))
            })
        } else {
            Ok(quote! {
                #(#all_names)|* => Ok(Self::#name)
            })
        }
    }

    pub fn gen_from_conf_serde_context_match_arm(
        &self,
        conf_context_ident: &Ident,
        next_value_producer_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        let name = self.get_name();
        let command_name = self.get_command_name();

        if self.get_serde_skip() {
            Ok(quote! {})
        } else if let Some(ty) = self.variant_type.as_ref() {
            let serde_name = self.get_serde_name();
            Ok(quote! {
                #command_name => {
                  let document_name = #conf_context_ident.document_name;
                  let seed = <#ty as ConfSerde>::Seed::from(ctxt);
                  Ok(Self::#name(#next_value_producer_ident.next_value_seed(seed).map_err(|err| {
                   vec![InnerError::serde(
                     document_name,
                     #serde_name,
                     err
                   )]
                  })??))
                }
            })
        } else {
            Ok(quote! {
                #command_name => Ok(Self::#name)
            })
        }
    }

    pub fn gen_push_parsers(
        &self,
        parsers_ident: &Ident,
        parsed_env_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        let command_name = &self.command_name;
        let aliases = &self.aliases;

        if let Some(ty) = self.variant_type.as_ref() {
            let inner_type = self.is_optional_type.as_ref().unwrap_or(ty);

            Ok(quote! {
              #parsers_ident.push(
                <#inner_type as ::conf::Conf>::get_parser(#parsed_env_ident)?
                  .rename(#command_name)
                  #(.add_alias(#aliases))*
              );
            })
        } else {
            Ok(quote! {
              #parsers_ident.push(
                ::conf::Parser::new(::conf::ParserConfig::default(), &[], &[], #parsed_env_ident)?
                  .rename(#command_name)
                  #(.add_alias(#aliases))*
              );
            })
        }
    }
}
