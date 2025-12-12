use crate::util::*;
use heck::{ToKebabCase, ToSnakeCase};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Error, Fields, FieldsNamed, FieldsUnnamed, Ident, LitStr, Type, Variant, meta::ParseNestedMeta,
    spanned::Spanned, token,
};

/// #[conf(serde(...))] options listed on a field of Flatten kind
pub struct VariantSerdeItem {
    pub rename: Option<LitStr>,
    pub aliases: Vec<LitStr>,
    pub skip: bool,
    span: Span,
}

impl VariantSerdeItem {
    pub fn new(meta: ParseNestedMeta<'_>) -> Result<Self, Error> {
        let mut result = Self {
            rename: None,
            aliases: Vec::new(),
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
                } else if path.is_ident("alias") {
                    result.aliases.push(parse_required_value::<LitStr>(meta)?);
                    Ok(())
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
    enum_name: Ident,
    variant_name: Ident,
    variant_type: Option<Type>, // None when we have a unit variant or named fields, Some otherwise
    is_optional_type: Option<Type>, // Some when we have a single unnamed field which is Option<T>
    /// For variants with named fields, store the fields for struct generation
    named_fields: Option<FieldsNamed>,
    command_name: LitStr,
    aliases: Vec<LitStr>,
    serde: Option<VariantSerdeItem>,
    doc_string: Option<String>,
    /// Struct-level conf attributes to pass through to the generated struct
    /// (e.g., one_of_fields, validation_predicate, etc.)
    passthrough_conf_attrs: Vec<TokenStream>,
}

impl VariantItem {
    pub fn new(variant: &Variant, enum_ident: &Ident) -> Result<Self, Error> {
        let enum_name = enum_ident.clone();
        let variant_name = variant.ident.clone();

        let (variant_type, is_optional_type, named_fields) = match variant.fields {
            Fields::Unit => (None, None, None),
            Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => {
                match unnamed.len() {
                    //0 => (None, None),
                    1 => {
                        let field = unnamed.first().unwrap();

                        let variant_type = field.ty.clone();
                        let is_optional_type = type_is_option(&variant_type)?;

                        (Some(variant_type), is_optional_type, None)
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
            Fields::Named(ref named) => {
                // Store the named fields for later struct generation
                (None, None, Some(named.clone()))
            }
        };

        let mut result = Self {
            command_name: LitStr::new(
                &variant_name.to_string().to_kebab_case(),
                variant_name.span(),
            ),
            enum_name,
            variant_name,
            variant_type,
            is_optional_type,
            named_fields,
            aliases: Vec::new(),
            serde: None,
            doc_string: None,
            passthrough_conf_attrs: Vec::new(),
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
                    } else if result.named_fields.is_some() {
                        // Pass through unrecognized attributes to the generated struct
                        // (e.g., one_of_fields, validation_predicate, etc.)
                        // Only valid for named-field variants which generate a struct.
                        let args: TokenStream = meta.input.parse()?;
                        result.passthrough_conf_attrs.push(quote! { #path #args });
                        Ok(())
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

    /// Get a mangled name for the generated struct (for named field variants).
    /// This uses a prefix to avoid shadowing types that might be referenced in field types.
    pub fn get_generated_struct_name(&self) -> Ident {
        Ident::new(
            &format!("__{}_{}", self.enum_name, self.variant_name),
            self.variant_name.span(),
        )
    }

    /// Get a user-friendly display name for error messages (e.g., "EnumName::VariantName").
    pub fn get_display_name(&self) -> String {
        format!("{}::{}", self.enum_name, self.variant_name)
    }

    /// Generate #[conf(...)] attributes to pass through to the generated struct
    pub fn gen_passthrough_conf_attrs(&self) -> TokenStream {
        let attrs = self.passthrough_conf_attrs.iter();
        quote! { #( #[conf(#attrs)] )* }
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

    pub fn get_serde_aliases(&self) -> Vec<LitStr> {
        self.serde
            .as_ref()
            .map(|serde| serde.aliases.clone())
            .unwrap_or_default()
    }

    /// Returns the named fields if this variant has named fields
    pub fn get_named_fields(&self) -> Option<&FieldsNamed> {
        self.named_fields.as_ref()
    }

    pub fn gen_from_conf_context_match_arm(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        let name = self.get_name();
        let all_names = self.get_all_command_names();

        if let Some(ty) = self.variant_type.as_ref() {
            // Single unnamed field variant
            Ok(quote! {
                #(#all_names)|* => Ok(Self::#name(<#ty as Conf>::from_conf_context(#conf_context_ident)?))
            })
        } else if let Some(fields) = &self.named_fields {
            // Named fields variant - use generated struct, then destructure
            let struct_name = self.get_generated_struct_name();
            let field_names: Vec<_> = fields
                .named
                .iter()
                .map(|f| f.ident.as_ref().unwrap())
                .collect();
            Ok(quote! {
                #(#all_names)|* => {
                    let __generated = <#struct_name as Conf>::from_conf_context(#conf_context_ident)?;
                    Ok(Self::#name { #( #field_names: __generated.#field_names ),* })
                }
            })
        } else {
            // Unit variant
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
            // Single unnamed field variant
            let serde_name = self.get_serde_name();
            Ok(quote! {
                #command_name => {
                  let document_name = #conf_context_ident.document_name;
                  let seed = ::conf::ConfSerdeSeed::<#ty>::from(ctxt);
                  Ok(Self::#name(#next_value_producer_ident.next_value_seed(seed).map_err(|err| {
                   vec![InnerError::serde(
                     document_name,
                     #serde_name,
                     err
                   )]
                  })??))
                }
            })
        } else if let Some(fields) = &self.named_fields {
            // Named fields variant - use generated struct, then destructure
            let struct_name = self.get_generated_struct_name();
            let serde_name = self.get_serde_name();
            let field_names: Vec<_> = fields
                .named
                .iter()
                .map(|f| f.ident.as_ref().unwrap())
                .collect();
            Ok(quote! {
                #command_name => {
                    let document_name = #conf_context_ident.document_name;
                    let seed = ::conf::ConfSerdeSeed::<#struct_name>::from(ctxt);
                    let __generated = #next_value_producer_ident.next_value_seed(seed).map_err(|err| {
                        vec![InnerError::serde(
                            document_name,
                            #serde_name,
                            err
                        )]
                    })??;
                    Ok(Self::#name { #( #field_names: __generated.#field_names ),* })
                }
            })
        } else {
            // Unit variant
            Ok(quote! {
                #command_name => Ok(Self::#name)
            })
        }
    }

    pub fn gen_push_parser(
        &self,
        parsers_ident: &Ident,
        parsed_env_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        let command_name = &self.command_name;
        let aliases = &self.aliases;

        // Turn first line of doc string into a summary for clap's "about"
        let about_lit: Option<LitStr> = self.doc_string.as_ref().and_then(|s| {
            let first_line = s.lines().map(|l| l.trim()).find(|l| !l.is_empty())?;
            let truncated: String = first_line.chars().take(200).collect();
            if truncated.is_empty() {
                None
            } else {
                Some(LitStr::new(&truncated, self.variant_name.span()))
            }
        });

        let about_ts = if let Some(about) = about_lit.as_ref() {
            quote! { .about(#about) }
        } else {
            quote! {}
        };

        if let Some(ty) = self.variant_type.as_ref() {
            // Single unnamed field variant
            let inner_type = self.is_optional_type.as_ref().unwrap_or(ty);

            Ok(quote! {
                {
                  let program_options = <#inner_type as ::conf::Conf>::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();
                  #parsers_ident.push(
                    <#inner_type as ::conf::Conf>::get_parser(#parsed_env_ident, program_options)?
                      .rename(#command_name)
                      #about_ts
                      #(.add_alias(#aliases))*
                  );
                }
            })
        } else if self.named_fields.is_some() {
            // Named fields variant - use the generated struct
            let struct_name = self.get_generated_struct_name();
            Ok(quote! {
                {
                    let program_options = <#struct_name as ::conf::Conf>::PROGRAM_OPTIONS.iter().collect::<Vec<_>>();
                    #parsers_ident.push(
                        <#struct_name as ::conf::Conf>::get_parser(#parsed_env_ident, program_options)?
                            .rename(#command_name)
                            #about_ts
                            #(.add_alias(#aliases))*
                    );
                }
            })
        } else {
            // Unit variant
            Ok(quote! {
              #parsers_ident.push(
                ::conf::Parser::new(::conf::ParserConfig::default(), vec![], &[], #parsed_env_ident)?
                  .rename(#command_name)
                  #about_ts
                  #(.add_alias(#aliases))*
              );
            })
        }
    }

    /// Generate debug assertions for this variant
    /// Call both parser_debug_asserts and debug_asserts on the inner Conf type (if any)
    pub fn gen_debug_asserts(&self) -> Result<TokenStream, Error> {
        if let Some(inner_type) = &self.variant_type {
            // Single unnamed field variant
            let conf_type = if let Some(opt_type) = &self.is_optional_type {
                opt_type.clone()
            } else {
                inner_type.clone()
            };

            Ok(quote! {
                <#conf_type as ::conf::Conf>::parser_debug_asserts();
                <#conf_type as ::conf::Conf>::debug_asserts();
            })
        } else if self.named_fields.is_some() {
            // Named fields variant - use the generated struct
            let struct_name = self.get_generated_struct_name();
            Ok(quote! {
                <#struct_name as ::conf::Conf>::parser_debug_asserts();
                <#struct_name as ::conf::Conf>::debug_asserts();
            })
        } else {
            // Unit variants don't have an inner Conf type, so nothing to check
            Ok(quote! {})
        }
    }
}
