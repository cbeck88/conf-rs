use super::StructItem;
use crate::util::type_is_bool;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Error, Expr, Field, Ident, LitStr, Meta, Path, Token, Type, parse_quote, punctuated::Punctuated,
};

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

/// Type of value parser - indicates whether it takes &str or &OsStr
pub enum ValueParserExpr {
    /// Parser takes &str (most common case)
    Str(Expr),
    /// Parser takes &OsStr (for PathBuf, OsString, or explicit value_parser_os)
    OsStr(Expr),
}

/// Indicates the type of data that an expr needs to produce in some setting.
pub enum ExprRequest {
    /// We need an expr manipulating str
    Str,
    /// We need an expr manipulating OsStr
    OsStr,
}

/// Indicates the strategy used for serde deserialization
///
/// To perform deserialization of structs with ConfSerde,
/// each field has a "state machine".
///
/// Given a map access, we enter a loop of the form:
///
/// while let Some(key) = ma.next_key::<...>()? {
///   match key.as_str() {
///
///   }
/// }
///
/// During this loop, a local variable whose name matches each field is in scope,
/// and it's value is Option<StateMachine>, initially None.
///
/// In the simplest case, StateMachine is simply Option<#field_type>, and
/// initialization happens in one shot, producing #field_type, or failing and
/// producing an Error, which goes to the error buffer.
/// In the most complex case, it's something implementing InitializationStateMachine.
///
/// After this loop, we must "finalize" any state machines, converting every variable
/// #field_name to have type Option<Option<#field_type>> and gathering errors.
///
/// Then, any values that are still None, meaning they weren't visited as we traversed
/// the serde document, should be initialized using the non-serde code path.
/// This step uses an unwrap-or-else and invokes the codegen from the no-serde version.
/// At that point, every #field_name is Option<#field_type>. We use the same "gather"
/// routine used in the non-serde path to reconstitute the struct, run validations, and
/// handle errors from there.
///
/// For any field, a SerdeStrategy has to tell us:
/// * What is the type of "StateMachine"
/// * What match arm should we use
/// * How do we finalize this state machine
///
/// Finally, if there are errors, we return the errors, otherwise we use a match expression
/// to unwrap all these options and initialize the struct. We also run any validation
/// predicates, and ultimately return it if everything is okay.
///
/// The SerdeStrategy carries all this data in one collection, to help ensure that it is computed
/// in a cohesive way.
pub struct SerdeStrategy {
    /// The state machine type. Typically just Option<#field_type>, even for serde(skip).
    pub state_machine_type: Type,
    /// The match arm to use
    pub match_arm: TokenStream,
    /// Whether to call `IninitializationStateMachine::finalize` or not
    pub has_finalizer: bool,
    /// The serde keys. This MUST match the values matched in the match arm.
    pub serde_keys: SerdeKeys,
    /// Serde keys to advertise in help messages. Doesn't include aliases.
    pub serde_help_keys: SerdeKeys,
}

impl SerdeStrategy {
    /// Used for serde(skip) fields
    pub fn skip(field: &FieldItem) -> Self {
        let ty = field.get_field_type();
        Self {
            state_machine_type: parse_quote! { Option<#ty> },
            match_arm: quote! {},
            has_finalizer: false,
            serde_keys: SerdeKeys::Lit(vec![]),
            serde_help_keys: SerdeKeys::Lit(vec![]),
        }
    }
}

#[derive(Clone)]
pub enum SerdeKeys {
    Lit(Vec<LitStr>),
    Expr(Expr),
}

/// #[conf(...)] options listed in a field of a struct which has `#[derive(Conf)]`
#[allow(clippy::large_enum_variant)]
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
    /// * conf_context_ident is a variable of type &ConfContext which is in scope
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
            match #field_name(#conf_context_ident) {
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
    /// * Ident for &conf_serde_context which is in scope and may be consumed
    /// * Ident for NextValueProducer object which is in scope and may be consumed
    /// * Ident for NextValueProducer type
    /// * Ident for errors buffer which is in scope, to which we may push.
    pub fn gen_serde_strategy(
        &self,
        ctxt: &Ident,
        nvp: &Ident,
        nvp_type: &Ident,
        errors_ident: &Ident,
    ) -> Result<SerdeStrategy, Error> {
        if self.get_serde_skip() {
            return Ok(SerdeStrategy::skip(self));
        }
        match self {
            Self::Flag(_) | Self::Parameter(_) | Self::Repeat(_) => {
                self.gen_simple_serde_strategy(ctxt, nvp, nvp_type, errors_ident)
            }
            Self::Flatten(item) => item.gen_serde_strategy(ctxt, nvp, nvp_type, errors_ident),
            Self::Subcommands(item) => item.gen_serde_strategy(ctxt, nvp, nvp_type, errors_ident),
        }
    }

    // A simple serde match arm is used at a "terminal", i.e. a flag, parameter, or repeat field.
    // These are fields that represent only a single program option.
    // This is a terminal in the sense that we don't recurse further using `DeserializeSeed` and our
    // own traits.
    //
    // The field has controls on what happens in this match arm via:
    // * get_serde_name()
    // * get_serde_aliases()
    // * get_serde_type()
    // * get_serde_deserialize_with()
    // * gen_initializer_with_doc_val()
    fn gen_simple_serde_strategy(
        &self,
        ctxt: &Ident,
        nvp: &Ident,
        nvp_type: &Ident,
        errors_ident: &Ident,
    ) -> Result<SerdeStrategy, Error> {
        let field_name = self.get_field_name();
        let field_name_str = field_name.to_string();

        let serde_name_str = self.get_serde_name();
        let serde_aliases = self.get_serde_aliases();
        let serde_type = self.get_serde_type();
        let deserialize_with = self.get_serde_deserialize_with();

        let conf_context_ident = Ident::new("__conf_context__", Span::call_site());
        let doc_name_ident = Ident::new("__doc_name__", Span::call_site());
        let doc_val_ident = Ident::new("__doc_val__", Span::call_site());
        let initializer = self.gen_initialize_from_conf_context_and_doc_val_and_push_errors(
            &conf_context_ident,
            &doc_name_ident,
            &doc_val_ident,
            errors_ident,
        )?;

        // Build match pattern: "name" | "alias1" | "alias2" => { ... }
        let match_pattern = if serde_aliases.is_empty() {
            quote! { #serde_name_str }
        } else {
            quote! { #serde_name_str | #(#serde_aliases)|* }
        };

        // Generate the deserialization call - either using deserialize_with or the default Deserialize impl
        let deserialize_call = if let Some(deserialize_with_path) = deserialize_with {
            quote! {
              {
                struct __DeserializeWith;
                impl<'dedede2> ::serde::de::DeserializeSeed<'dedede2> for __DeserializeWith {
                  type Value = #serde_type;
                  fn deserialize<__D>(self, __deserializer: __D) -> ::std::result::Result<Self::Value, __D::Error>
                  where
                    __D: ::serde::de::Deserializer<'dedede2>
                  {
                    (#deserialize_with_path)(__deserializer)
                  }
                }
                #nvp.next_value_seed(__DeserializeWith)
              }
            }
        } else {
            quote! {
              #nvp.next_value::<#serde_type>()
            }
        };

        let match_arm = quote! {
          #match_pattern => {
            if #field_name.is_some() {
              #errors_ident.push(
                InnerError::serde(
                  #ctxt.document_name,
                  #field_name_str,
                  #nvp_type::Error::duplicate_field(#serde_name_str)
                )
              );
            } else {
              #field_name = Some(match #deserialize_call {
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

        // Return all names for error messages
        let mut all_names = vec![serde_name_str.clone()];
        all_names.extend(serde_aliases);

        let field_type = self.get_field_type();
        Ok(SerdeStrategy {
            state_machine_type: parse_quote! { Option<#field_type> },
            match_arm,
            has_finalizer: false,
            serde_keys: SerdeKeys::Lit(all_names),
            serde_help_keys: SerdeKeys::Lit(vec![serde_name_str]),
        })
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

    /// Get the serde aliases (only when "is_single_option" is true)
    fn get_serde_aliases(&self) -> Vec<LitStr> {
        match self {
            Self::Flag(item) => item.get_serde_aliases(),
            Self::Parameter(item) => item.get_serde_aliases(),
            Self::Repeat(item) => item.get_serde_aliases(),
            Self::Flatten(_item) => unimplemented!(),
            Self::Subcommands(_item) => unimplemented!(),
        }
    }

    /// Get the serde deserialize_with expression (only when "is_single_option" is true)
    fn get_serde_deserialize_with(&self) -> Option<Path> {
        match self {
            Self::Flag(item) => item.get_serde_deserialize_with(),
            Self::Parameter(item) => item.get_serde_deserialize_with(),
            Self::Repeat(item) => item.get_serde_deserialize_with(),
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

    /// Generate debug assertions for this field
    pub fn gen_debug_asserts(&self, struct_ident: &Ident) -> Result<TokenStream, Error> {
        match self {
            Self::Flag(item) => item.gen_debug_asserts(struct_ident),
            Self::Parameter(item) => item.gen_debug_asserts(struct_ident),
            Self::Repeat(item) => item.gen_debug_asserts(struct_ident),
            Self::Flatten(item) => item.gen_debug_asserts(struct_ident),
            Self::Subcommands(item) => item.gen_debug_asserts(struct_ident),
        }
    }
}
