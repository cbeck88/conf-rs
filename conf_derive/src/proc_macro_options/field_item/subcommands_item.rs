use super::{SerdeKeys, SerdeStrategy, StructItem};
use crate::util::*;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Error, Field, Ident, Lifetime, Type, meta::ParseNestedMeta, spanned::Spanned, token};

/// #[conf(serde(...))] options listed on a field of Subcommands kind
pub struct SubcommandsSerdeItem {
    pub skip: bool,
    span: Span,
}

impl SubcommandsSerdeItem {
    pub fn new(meta: ParseNestedMeta<'_>) -> Result<Self, Error> {
        let mut result = Self {
            skip: false,
            span: meta.input.span(),
        };

        if meta.input.peek(token::Paren) {
            meta.parse_nested_meta(|meta| {
                let path = meta.path.clone();
                if path.is_ident("skip") {
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

impl GetSpan for SubcommandsSerdeItem {
    fn get_span(&self) -> Span {
        self.span
    }
}

/// #[conf(...)] options listed on a field of Subcommands kind
pub struct SubcommandsItem {
    struct_name: Ident,
    field_name: Ident,
    field_type: Type,
    is_optional_type: Option<Type>,
    serde: Option<SubcommandsSerdeItem>,
    doc_string: Option<String>,
}

impl SubcommandsItem {
    pub fn new(field: &Field, struct_item: &StructItem) -> Result<Self, Error> {
        let struct_name = struct_item.struct_ident.clone();
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "missing identifier"))?;
        let field_type = field.ty.clone();
        let is_optional_type = type_is_option(&field.ty)?;

        let mut result = Self {
            struct_name,
            field_name,
            field_type,
            is_optional_type,
            serde: None,
            doc_string: None,
        };

        for attr in &field.attrs {
            maybe_append_doc_string(&mut result.doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("subcommands") {
                        Ok(())
                    } else if path.is_ident("serde") {
                        set_once(
                            &path,
                            &mut result.serde,
                            Some(SubcommandsSerdeItem::new(meta)?),
                        )
                    } else {
                        Err(meta.error("unrecognized conf subcommands option"))
                    }
                })?;
            }
        }

        Ok(result)
    }

    pub fn get_field_name(&self) -> &Ident {
        &self.field_name
    }

    pub fn get_field_type(&self) -> Type {
        self.field_type.clone()
    }

    pub fn get_serde_skip(&self) -> bool {
        self.serde.as_ref().map(|serde| serde.skip).unwrap_or(false)
    }

    // Subcommands fields don't contribute nodes to PROGRAM_OPTIONS.
    // They are filtered out via returning None here.
    pub fn gen_program_option_node(&self) -> Result<Option<TokenStream>, Error> {
        Ok(None)
    }

    // Subcommands fields add subcommand parsers to the conf structure.
    pub fn gen_push_subcommands(
        &self,
        parsers_ident: &Ident,
        parsed_env_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        let inner_type: &Type = self.is_optional_type.as_ref().unwrap_or(&self.field_type);
        let panic_message = format!(
            "Not supported to have multiple subcommands fields on the same struct: at field '{}'",
            self.field_name
        );

        // TODO: In theory we could support having multiple subcommands fields on the same struct,
        // but it's not clear that it's useful.
        Ok(quote! {
            if !#parsers_ident.is_empty() {
              panic!(#panic_message);
            }
            #parsers_ident.extend(
              <#inner_type as ::conf::Subcommands>::get_parsers(#parsed_env_ident)?
            );
        })
    }

    // Body of a function taking a &ConfContext and returning
    // Result<#field_type, Vec<::conf::InnerError>>
    //
    // Arguments:
    // * conf_context_ident is the identifier of a &ConfContext variable in scope
    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        let struct_name = self.struct_name.to_string();
        let field_name = self.field_name.to_string();
        let field_type = &self.field_type;

        let initializer = if let Some(inner_type) = self.is_optional_type.as_ref() {
            quote! {
              Ok(if let Some((name, conf_context)) = #conf_context_ident.for_subcommand() {
                Some(
                  <#inner_type as ::conf::Subcommands>::from_conf_context(name, conf_context)?
                )
              } else {
                None
              })
            }
        } else {
            quote! {
              use ::conf::{InnerError, Subcommands};
              let Some((name, conf_context)) = #conf_context_ident.for_subcommand() else {
                return Err(vec![
                  InnerError::missing_required_subcommand(
                    #struct_name,
                    #field_name,
                    <#field_type as Subcommands>::get_subcommand_names()
                  )
                ]);
              };
              <#field_type as Subcommands>::from_conf_context(name, conf_context)
            }
        };

        Ok((initializer, true))
    }

    // A serde strategy for the subcommand.
    pub fn gen_serde_strategy(
        &self,
        _ct: &Lifetime,
        ctxt: &Ident,
        nvp: &Ident,
        nvp_type: &Ident,
        errors_ident: &Ident,
    ) -> Result<SerdeStrategy, Error> {
        let field_name = self.get_field_name();
        let inner_type = self.is_optional_type.as_ref().unwrap_or(&self.field_type);

        let val_expr = if self.is_optional_type.is_some() {
            quote! { Some(__val__) }
        } else {
            quote! { __val__ }
        };

        // For a subcommand to be active, it must appear in the args, we can't activate a subcommand
        // based only on the conf file.
        // We'd like to allow that each subcommand could have its own section in the conf file, and
        // sections that don't correspond to the currently selected one aren't an error.
        // Multiple subcommands could have the same section as well (if their serde name were
        // equal).
        //
        // So the test is:
        // * If this key matches any serde_name for any of the subcommands, enter this match arm.
        // * Check if the conf context has a subcommand name, and if that matches any of these
        //   commands. If not, then we ignore this serde value.
        // * Otherwise, we are attempting to recurse into the subcommand.
        let match_pattern = quote! {
            key__ if <#inner_type as SubcommandsSerde>::SERDE_NAMES.iter().any(|(_c, s)| *s == key__)
        };
        let match_expr = quote! {
          {
            // Get the active subcommand from context, or skip if none
            let Some((command_name, conf_context_serde)) = #ctxt.for_subcommand() else { break 'match_statement };

            // Find the subcommand entry that matches BOTH the active command AND this key.
            // This ensures we only process this key if it corresponds to the active subcommand.
            // E.g., if key is "b" but active command is "a", the find returns None and we skip.
            let Some((static_command_name, static_serde_name)) = <#inner_type as SubcommandsSerde>::SERDE_NAMES.iter().find(|(c, s)| *c == command_name && *s == key__) else { break 'match_statement };

            if #field_name.is_some() {
              #errors_ident.push(
                InnerError::serde(
                  #ctxt.document_name,
                  static_command_name,
                  #nvp_type::Error::duplicate_field(static_serde_name)
                )
              );
            } else {
              #field_name = Some(match <#inner_type as SubcommandsSerde>::from_conf_serde_context(&command_name, conf_context_serde, #nvp) {
                Ok(__val__) => {
                  Some(#val_expr)
                },
                Err(__errs__) => {
                  #errors_ident.extend(__errs__);
                  None
                }
              });
            }
          },
        };
        // We don't know the SERDE_NAMES as string literals in this proc_macro, they are only in the
        // proc_macro invocation for the enum.
        Ok(SerdeStrategy {
            state_machine_type: None,
            state_machine_init: None,
            serde_keys: SerdeKeys::Expr(match_pattern),
            match_expr,
        })
    }

    /// Generate debug assertions for this subcommands field
    /// Call debug_asserts on the subcommands enum
    pub fn gen_debug_asserts(&self, _struct_ident: &Ident) -> Result<TokenStream, Error> {
        let enum_type = if let Some(opt_type) = &self.is_optional_type {
            opt_type.clone()
        } else {
            self.field_type.clone()
        };

        Ok(quote! {
            <#enum_type as ::conf::Subcommands>::debug_asserts();
        })
    }
}
