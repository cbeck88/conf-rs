use crate::util::*;
use heck::{ToKebabCase, ToSnakeCase};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    meta::ParseNestedMeta, spanned::Spanned, token, Error, Fields, FieldsUnnamed, Ident, LitStr,
    Type, Variant,
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
    variant_type: Type,
    is_optional_type: Option<Type>,
    command_name: LitStr,
    serde: Option<VariantSerdeItem>,
    doc_string: Option<String>,
}

impl VariantItem {
    pub fn new(variant: &Variant, _enum_ident: &Ident) -> Result<Self, Error> {
        let Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) = variant.fields else {
            return Err(Error::new(
                variant.fields.span(),
                "Subcommands variant must contain a single unnamed field which implements Conf",
            ));
        };
        if unnamed.len() != 1 {
            return Err(Error::new(
                unnamed.span(),
                "Subcommands variant must contain a single unnamed field which implements Conf",
            ));
        }
        let field = unnamed.first().unwrap();

        let variant_name = variant.ident.clone();
        let variant_type = field.ty.clone();
        let is_optional_type = type_is_option(&variant_type)?;

        let mut result = Self {
            command_name: LitStr::new(
                &variant_name.to_string().to_kebab_case(),
                variant_name.span(),
            ),
            variant_name,
            variant_type,
            is_optional_type,
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

    pub fn get_type(&self) -> Type {
        self.variant_type.clone()
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

    pub fn gen_push_parsers(
        &self,
        parsers_ident: &Ident,
        parsed_env_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        let inner_type = self.is_optional_type.as_ref().unwrap_or(&self.variant_type);
        let command_name = &self.command_name;

        Ok(quote! {
          #parsers_ident.push(
            <#inner_type as ::conf::Conf>::get_parser(#parsed_env_ident)?
              .rename(#command_name)
          );
        })
    }
}
