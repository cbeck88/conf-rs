use super::StructItem;
use crate::util::type_is_bool;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{punctuated::Punctuated, Error, Field, Ident, LitStr, Meta, Token, Type};

mod flag_item;
mod flatten_item;
mod parameter_item;
mod repeat_item;
mod subcommands_item;

use flag_item::FlagItem;
use flatten_item::FlattenItem;
use parameter_item::ParameterItem;
use repeat_item::RepeatItem;
use subcommands_item::SubcommandsItem;

/// #[conf(...)] options listed in a field of a struct which has `#[derive(Conf)]`
pub enum FieldItem {
    Flag(FlagItem),
    Parameter(ParameterItem),
    Repeat(RepeatItem),
    Flatten(FlattenItem),
    Subcommands(SubcommandsItem),
}

impl FieldItem {
    pub fn new(field: &Field, struct_item: &StructItem) -> Result<Self, Error> {
        // First, inspect the first field attribute.
        // If the first attribute is 'flag', 'parameter', 'repeat', or 'flatten', then that's how
        // we're going to handle it.
        for attr in &field.attrs {
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
                let nested =
                    attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
                if let Some(meta) = nested.first() {
                    let path = meta.path();
                    if path.is_ident("flag") {
                        return Ok(Self::Flag(FlagItem::new(field, struct_item)?));
                    } else if path.is_ident("parameter") {
                        return Ok(Self::Parameter(ParameterItem::new(field, struct_item)?));
                    } else if path.is_ident("repeat") {
                        return Ok(Self::Repeat(RepeatItem::new(field, struct_item)?));
                    } else if path.is_ident("flatten") {
                        return Ok(Self::Flatten(FlattenItem::new(field, struct_item)?));
                    } else if path.is_ident("subcommands") {
                        return Ok(Self::Subcommands(SubcommandsItem::new(field, struct_item)?));
                    }
                }
            }
        }

        // We're still not sure, so inspect the type.
        // If it's bool, it's a flag. Otherwise it's a parameter.
        Ok(if type_is_bool(&field.ty) {
            Self::Flag(FlagItem::new(field, struct_item)?)
        } else {
            Self::Parameter(ParameterItem::new(field, struct_item)?)
        })
    }

    /// True if this field represents a single program option
    pub fn is_single_option(&self) -> bool {
        matches!(
            self,
            Self::Flag(..) | Self::Parameter(..) | Self::Repeat(..)
        )
    }

    /// Get the field name
    pub fn get_field_name(&self) -> &Ident {
        match self {
            Self::Flag(item) => item.get_field_name(),
            Self::Parameter(item) => item.get_field_name(),
            Self::Repeat(item) => item.get_field_name(),
            Self::Flatten(item) => item.get_field_name(),
            Self::Subcommands(item) => item.get_field_name(),
        }
    }

    /// Get the field type
    pub fn get_field_type(&self) -> Type {
        match self {
            Self::Flag(item) => item.get_field_type(),
            Self::Parameter(item) => item.get_field_type(),
            Self::Repeat(item) => item.get_field_type(),
            Self::Flatten(item) => item.get_field_type(),
            Self::Subcommands(item) => item.get_field_type(),
        }
    }

    /// Generate code that constructs (one or more) ProgramOption as needed and pushes them onto
    /// program_options_ident
    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        match self {
            Self::Flag(item) => item.gen_push_program_options(program_options_ident),
            Self::Parameter(item) => item.gen_push_program_options(program_options_ident),
            Self::Repeat(item) => item.gen_push_program_options(program_options_ident),
            Self::Flatten(item) => item.gen_push_program_options(program_options_ident),
            Self::Subcommands(item) => item.gen_push_program_options(program_options_ident),
        }
    }

    /// Generate code that constructs (one or more) subcommands as needed and pushes them onto
    /// subcommands_ident
    pub fn gen_push_subcommands(
        &self,
        subcommands_ident: &Ident,
        parsed_env: &Ident,
    ) -> Result<TokenStream, Error> {
        match self {
            Self::Flag(item) => item.gen_push_subcommands(subcommands_ident, parsed_env),
            Self::Parameter(item) => item.gen_push_subcommands(subcommands_ident, parsed_env),
            Self::Repeat(item) => item.gen_push_subcommands(subcommands_ident, parsed_env),
            Self::Flatten(item) => item.gen_push_subcommands(subcommands_ident, parsed_env),
            Self::Subcommands(item) => item.gen_push_subcommands(subcommands_ident, parsed_env),
        }
    }

    /// Generate code for a struct initializer for this field, reading from conf_context
    ///
    /// Returns:
    /// * a TokenStream for initializer expression, which can use `?` to return errors,
    /// * a bool which is true if the error type is `Vec<InnerError>` and false if it is
    ///   `InnerError`
    fn gen_initializer(&self, conf_context_ident: &Ident) -> Result<(TokenStream, bool), Error> {
        match self {
            Self::Flag(item) => item.gen_initializer(conf_context_ident),
            Self::Parameter(item) => item.gen_initializer(conf_context_ident),
            Self::Repeat(item) => item.gen_initializer(conf_context_ident),
            Self::Flatten(item) => item.gen_initializer(conf_context_ident),
            Self::Subcommands(item) => item.gen_initializer(conf_context_ident),
        }
    }

    /// Generate code for a struct initializer for this field, reading from conf_context
    ///
    /// Returns:
    /// * a TokenStream for initializer expression, which can use `?` to return errors,
    /// * a bool which is true if the error type is `Vec<InnerError>` and false if it is
    ///   `InnerError`
    fn gen_initializer_with_doc_val(
        &self,
        conf_context_ident: &Ident,
        doc_name_ident: &Ident,
        doc_val_ident: &Ident,
    ) -> Result<(TokenStream, bool), Error> {
        match self {
            Self::Flag(item) => {
                item.gen_initializer_with_doc_val(conf_context_ident, doc_name_ident, doc_val_ident)
            }
            Self::Parameter(item) => {
                item.gen_initializer_with_doc_val(conf_context_ident, doc_name_ident, doc_val_ident)
            }
            Self::Repeat(item) => {
                item.gen_initializer_with_doc_val(conf_context_ident, doc_name_ident, doc_val_ident)
            }
            Self::Flatten(_item) => unimplemented!("uses a custom match arm"),
            Self::Subcommands(_item) => unimplemented!("would have to use a custom match arm"),
        }
    }

    /// Generate code of the form
    ///
    /// {
    ///   fn #field_name(conf_context: &...) -> Result<#field_type, InnerError> { .. }
    ///   match field_name(...) {
    ///     Ok(t) => Some(t),
    ///     Err(err) => { errors.push(err); None },
    ///   }
    /// }
    ///
    /// This value can then be assigned to a variable, e.g.
    ///
    /// let #field_name = #initializer;
    ///
    /// The code block reads from a conf context and pushes any errors to given errors buffer.
    ///
    /// Arguments:
    /// * conf_context_ident is a variable of type ConfContext which is in scope, which we won't
    ///   consume
    /// * errors_ident is a variable of type mut Vec<InnerError> which is in scope, which we can
    ///   push to.
    pub fn gen_initialize_from_conf_context_and_push_errors(
        &self,
        conf_context_ident: &Ident,
        errors_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        let field_name = self.get_field_name();
        let field_type = self.get_field_type();
        let (initializer, returns_multiple_errors) = self.gen_initializer(conf_context_ident)?;
        // The initializer is the portion e.g.
        // fn(conf_context: &...) -> Result<T, conf::InnerError> { ... }
        //
        // It returns `Result<T, Vec<conf::InnerError>>` if returns_multiple_errors is true,
        // otherwise it's Result<T, conf::InnerError> It is allowed to read
        // #conf_context_ident but not modify it We have to put it inside a locally defined
        // fn so that it cannot modify the errors buffer etc.

        let (error_type, extend_fn) = if returns_multiple_errors {
            (
                quote! { ::std::vec::Vec<::conf::InnerError> },
                quote! { extend },
            )
        } else {
            (quote! { ::conf::InnerError }, quote! { push })
        };

        Ok(quote! {
          {
            fn #field_name(
              #conf_context_ident: &::conf::ConfContext<'_>
            ) -> Result<#field_type, #error_type> {
              #initializer
            }
            match #field_name(&#conf_context_ident) {
              Ok(val) => Some(val),
              Err(errs) => {
                #errors_ident.#extend_fn(errs);
                None
              }
            }
          }
        })
    }

    /// Generate code of the form
    ///
    /// {
    ///   fn #field_name(c: &ConfContext, d: ...) -> Result<#field_type, Vec<InnerError> {
    ///      ..
    ///   }
    ///   match #field_name(...) {
    ///     Ok(val) => Some(val),
    ///     Err(err) => #errors_ident.extend(err);
    ///   }
    /// }
    ///
    /// which can then be assigned to a variable, e.g. #field_name
    ///
    /// The code block reads from a conf context and a value from document, resolves it, and pushes
    /// any errors to given errors ident.
    ///
    /// Arguments:
    /// * conf_context_ident is a variable of type ConfContext which is in scope, which we won't
    ///   consume
    /// * doc_name_ident is a variable of type &str, which is the name of the document this value
    ///   came from
    /// * doc_val_ident is a variable of type #serde_type, which was parsed from serde successfully.
    /// * errors_ident is a variable of type mut Vec<InnerError> which is in scope, which we can
    ///   push to.
    pub fn gen_initialize_from_conf_context_and_doc_val_and_push_errors(
        &self,
        conf_context_ident: &Ident,
        doc_name_ident: &Ident,
        doc_val_ident: &Ident,
        errors_ident: &Ident,
    ) -> Result<TokenStream, Error> {
        let field_name = self.get_field_name();
        let field_type = self.get_field_type();
        let serde_type = self.get_serde_type();
        let (initializer, returns_multiple_errors) =
            self.gen_initializer_with_doc_val(conf_context_ident, doc_name_ident, doc_val_ident)?;
        // The initializer is the portion e.g.
        // fn(conf_context: &..., doc_val: T) -> Result<T, conf::InnerError> { ... }
        //
        // It returns `Result<T, Vec<conf::InnerError>>` if returns_multiple_errors is true,
        // otherwise it's Result<T, conf::InnerError>
        // It is allowed to read #conf_context_ident but not modify it
        // We have to put it inside a locally defined
        // fn so that it cannot modify the errors buffer etc.

        let (error_type, extend_fn) = if returns_multiple_errors {
            (
                quote! { ::std::vec::Vec<::conf::InnerError> },
                quote! { extend },
            )
        } else {
            (quote! { ::conf::InnerError }, quote! { push })
        };

        Ok(quote! {
          {
            fn #field_name(
              #conf_context_ident: &::conf::ConfContext<'_>,
              #doc_name_ident: &str,
              #doc_val_ident: #serde_type
            ) -> Result<#field_type, #error_type> {
              #initializer
            }
            match #field_name(&#conf_context_ident, #doc_name_ident, #doc_val_ident) {
              Ok(val) => Some(val),
              Err(errs) => {
                #errors_ident.#extend_fn(errs);
                None
              }
            }
          }
        })
    }

    /// Generate (one or more) match arms for iterating a serde MapAccess object.
    /// The match takes place on a `&str` representing the struct key.
    /// The match arm(s) generated here should identify one or more `&str`
    /// and perform initialization appropriately.
    ///
    /// Returns a TokenStream for the match arm, and a list of `serde_name`'s which
    /// we want to advertise in error messages.
    ///
    /// Arguments:
    /// * Ident for conf_serde_context which is in scope and may be consumed
    /// * Ident for map_access object which is in scope and may be consumed
    /// * Ident for map_access type
    /// * Ident for errors buffer which is in scope, to which we may push.
    pub fn gen_serde_match_arm(
        &self,
        ctxt: &Ident,
        map_access: &Ident,
        map_access_type: &Ident,
        errors_ident: &Ident,
    ) -> Result<(TokenStream, Vec<LitStr>), Error> {
        if self.get_serde_skip() {
            return Ok((quote! {}, vec![]));
        }
        match self {
            Self::Flag(_) | Self::Parameter(_) | Self::Repeat(_) => {
                self.gen_simple_serde_match_arm(ctxt, map_access, map_access_type, errors_ident)
            }
            Self::Flatten(item) => {
                item.gen_serde_match_arm(ctxt, map_access, map_access_type, errors_ident)
            }
            Self::Subcommands(item) => {
                item.gen_serde_match_arm(ctxt, map_access, map_access_type, errors_ident)
            }
        }
    }

    // A simple serde match arm is used at a "terminal", i.e. a flag, parameter, or repeat field.
    // These are fields that represent only a single program option.
    // This is a terminal in the sense that we don't recurse further using `DeserializeSeed` and our
    // own traits.
    //
    // The field has controls on what happens in this match arm via:
    // * get_serde_name()
    // * get_serde_type()
    // * gen_initializer_with_doc_val()
    fn gen_simple_serde_match_arm(
        &self,
        ctxt: &Ident,
        map_access: &Ident,
        map_access_type: &Ident,
        errors_ident: &Ident,
    ) -> Result<(TokenStream, Vec<LitStr>), Error> {
        let field_name = self.get_field_name();
        let field_name_str = field_name.to_string();

        let serde_name_str = self.get_serde_name();
        let serde_type = self.get_serde_type();

        let conf_context_ident = Ident::new("__conf_context__", Span::call_site());
        let doc_name_ident = Ident::new("__doc_name__", Span::call_site());
        let doc_val_ident = Ident::new("__doc_val__", Span::call_site());
        let initializer = self.gen_initialize_from_conf_context_and_doc_val_and_push_errors(
            &conf_context_ident,
            &doc_name_ident,
            &doc_val_ident,
            errors_ident,
        )?;

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
              #field_name = Some(match #map_access.next_value::<#serde_type>() {
                Ok(#doc_val_ident) => {
                  let #conf_context_ident: &::conf::ConfContext = &#ctxt.conf_context;
                  let #doc_name_ident: &str = #ctxt.document_name;
                  #initializer
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

    /// Get the serde name (only when "is_single_option" is true)
    fn get_serde_name(&self) -> LitStr {
        match self {
            Self::Flag(item) => item.get_serde_name(),
            Self::Parameter(item) => item.get_serde_name(),
            Self::Repeat(item) => item.get_serde_name(),
            Self::Flatten(_item) => unimplemented!(),
            Self::Subcommands(_item) => unimplemented!(),
        }
    }

    /// Get the serde type (only when "is_single_option" is true)
    fn get_serde_type(&self) -> Type {
        match self {
            Self::Flag(item) => item.get_serde_type(),
            Self::Parameter(item) => item.get_serde_type(),
            Self::Repeat(item) => item.get_serde_type(),
            Self::Flatten(_item) => unimplemented!(),
            Self::Subcommands(_item) => unimplemented!(),
        }
    }

    /// Get the serde(skip) option
    fn get_serde_skip(&self) -> bool {
        match self {
            Self::Flag(item) => item.get_serde_skip(),
            Self::Parameter(item) => item.get_serde_skip(),
            Self::Repeat(item) => item.get_serde_skip(),
            Self::Flatten(item) => item.get_serde_skip(),
            Self::Subcommands(item) => item.get_serde_skip(),
        }
    }
}
