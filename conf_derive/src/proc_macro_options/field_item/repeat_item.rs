use super::{StructItem, ValueParserExpr};
use crate::util::*;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Error, Expr, Field, Ident, LitBool, LitChar, LitStr, Path, Type, meta::ParseNestedMeta,
    parse_quote, spanned::Spanned, token,
};

/// #[conf(serde(...))] options listed on a field of Repeat kind
pub struct RepeatSerdeItem {
    pub rename: Option<LitStr>,
    pub aliases: Vec<LitStr>,
    pub skip: bool,
    pub use_value_parser: bool,
    pub deserialize_with: Option<Path>,
    pub try_from: Option<Type>,
    span: Span,
}

impl RepeatSerdeItem {
    pub fn new(meta: ParseNestedMeta<'_>) -> Result<Self, Error> {
        let mut result = Self {
            rename: None,
            aliases: Vec::new(),
            skip: false,
            use_value_parser: false,
            deserialize_with: None,
            try_from: None,
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
                } else if path.is_ident("use_value_parser") {
                    result.use_value_parser = true;
                    Ok(())
                } else if path.is_ident("deserialize_with") {
                    set_once(
                        &path,
                        &mut result.deserialize_with,
                        Some(parse_path_from_str(meta)?),
                    )
                } else if path.is_ident("try_from") {
                    set_once(
                        &path,
                        &mut result.try_from,
                        Some(parse_type_from_str(meta)?),
                    )
                } else {
                    Err(meta.error("unrecognized conf(serde) option"))
                }
            })?;
        }

        // Validate mutual exclusivity
        if result.deserialize_with.is_some() && result.use_value_parser {
            return Err(Error::new(
                result.span,
                "deserialize_with and use_value_parser are mutually exclusive",
            ));
        }

        if result.try_from.is_some() && result.use_value_parser {
            return Err(Error::new(
                result.span,
                "try_from and use_value_parser are mutually exclusive",
            ));
        }

        if result.try_from.is_some() && result.deserialize_with.is_some() {
            return Err(Error::new(
                result.span,
                "try_from and deserialize_with are mutually exclusive",
            ));
        }

        Ok(result)
    }
}

impl GetSpan for RepeatSerdeItem {
    fn get_span(&self) -> Span {
        self.span
    }
}

/// Proc macro annotations parsed from a field of Repeat kind
pub struct RepeatItem {
    field_name: Ident,
    field_type: Type, // This is needed to help with type inference in code gen
    allow_hyphen_values: bool,
    secret: Option<LitBool>,
    short_switch: Option<LitChar>,
    long_switch: Option<LitStr>,
    aliases: Option<LitStrArray>,
    env_name: Option<LitStr>,
    env_aliases: Option<LitStrArray>,
    value_parser: Option<Expr>,
    value_parser_os: Option<Expr>,
    env_delimiter: Option<LitChar>,
    no_env_delimiter: bool,
    serde: Option<RepeatSerdeItem>,
    description: Option<String>,
    is_positional: bool,
}

impl RepeatItem {
    pub fn new(field: &Field, _struct_item: &StructItem) -> Result<Self, Error> {
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "missing identifier"))?;
        let field_type = field.ty.clone();

        let Some(inner_type) = type_is_vec(&field_type)? else {
            return Err(Error::new(
                field.ty.span(),
                "Type of a conf(repeat) field must be Vec<T>",
            ));
        };
        let allow_hyphen_values = type_is_signed_number(&inner_type);

        let mut result = Self {
            field_name,
            field_type,
            allow_hyphen_values,
            secret: None,
            short_switch: None,
            long_switch: None,
            aliases: None,
            env_name: None,
            env_aliases: None,
            value_parser: None,
            value_parser_os: None,
            env_delimiter: None,
            no_env_delimiter: false,
            serde: None,
            description: None,
            is_positional: false,
        };

        for attr in &field.attrs {
            maybe_append_doc_string(&mut result.description, &attr.meta)?;
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("repeat") {
                        Ok(())
                    } else if path.is_ident("short") {
                        set_once(
                            &path,
                            &mut result.short_switch,
                            parse_optional_value::<LitChar>(meta)?
                                .or(make_short(&result.field_name, path.span())),
                        )
                    } else if path.is_ident("long") {
                        set_once(
                            &path,
                            &mut result.long_switch,
                            parse_optional_value::<LitStr>(meta)?
                                .or(make_long(&result.field_name, path.span())),
                        )
                    } else if path.is_ident("aliases") {
                        set_once(
                            &path,
                            &mut result.aliases,
                            Some(parse_required_value::<LitStrArray>(meta)?),
                        )
                    } else if path.is_ident("env") {
                        set_once(
                            &path,
                            &mut result.env_name,
                            parse_optional_value::<LitStr>(meta)?
                                .or(make_env(&result.field_name, path.span())),
                        )
                    } else if path.is_ident("env_aliases") {
                        set_once(
                            &path,
                            &mut result.env_aliases,
                            Some(parse_required_value::<LitStrArray>(meta)?),
                        )
                    } else if path.is_ident("value_parser") {
                        set_once(
                            &path,
                            &mut result.value_parser,
                            Some(parse_required_value::<Expr>(meta)?),
                        )
                    } else if path.is_ident("value_parser_os") {
                        set_once(
                            &path,
                            &mut result.value_parser_os,
                            Some(parse_required_value::<Expr>(meta)?),
                        )
                    } else if path.is_ident("env_delimiter") {
                        set_once(
                            &path,
                            &mut result.env_delimiter,
                            Some(parse_required_value::<LitChar>(meta)?),
                        )
                    } else if path.is_ident("no_env_delimiter") {
                        result.no_env_delimiter = true;
                        Ok(())
                    } else if path.is_ident("allow_hyphen_values") {
                        result.allow_hyphen_values = true;
                        Ok(())
                    } else if path.is_ident("secret") {
                        set_once(
                            &path,
                            &mut result.secret,
                            Some(
                                parse_optional_value::<LitBool>(meta)?
                                    .unwrap_or(LitBool::new(true, path.span())),
                            ),
                        )
                    } else if path.is_ident("serde") {
                        set_once(&path, &mut result.serde, Some(RepeatSerdeItem::new(meta)?))
                    } else if path.is_ident("pos") {
                        result.is_positional = true;
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized conf repeat option"))
                    }
                })?;
            }
        }

        // Validate value_parser and value_parser_os are mutually exclusive
        if result.value_parser.is_some() && result.value_parser_os.is_some() {
            return Err(Error::new(
                field.span(),
                "Cannot specify both value_parser and value_parser_os",
            ));
        }

        if result.no_env_delimiter && result.env_delimiter.is_some() {
            return Err(Error::new(
                field.span(),
                "Cannot specify both env_delimiter and no_env_delimiter",
            ));
        }

        if result.env_delimiter.is_some() && result.env_name.is_none() {
            return Err(Error::new(
                field.span(),
                "env_delimiter has no effect if an env variable is not declared",
            ));
        }

        if result.no_env_delimiter && result.env_name.is_none() {
            return Err(Error::new(
                field.span(),
                "no_env_delimiter has no effect if an env variable is not declared",
            ));
        }

        // Validate that env_delimiter is ASCII when value_parser_os is used (explicitly or implicitly)
        // This is required because OsStr splitting only works safely with ASCII delimiters
        if let Some(ref delim) = result.env_delimiter {
            if matches!(result.get_value_parser_expr(), ValueParserExpr::OsStr(_))
                && !delim.value().is_ascii()
            {
                return Err(Error::new(
                    delim.span(),
                    "env_delimiter must be an ASCII character when using value_parser_os \
                     (or with Vec<PathBuf>/Vec<OsString> types). \
                     OsStr uses a platform-specific encoding and only splitting by ASCII is supported.",
                ));
            }
        }

        // Validate positional argument constraints
        if result.is_positional && result.long_switch.is_some() {
            return Err(Error::new(
                field.span(),
                "#[conf(pos)] cannot be used with #[conf(long)]",
            ));
        }

        if result.is_positional && result.short_switch.is_some() {
            return Err(Error::new(
                field.span(),
                "#[conf(pos)] cannot be used with #[conf(short)]",
            ));
        }

        if result.long_switch.is_none()
            && !result
                .aliases
                .as_ref()
                .map(LitStrArray::is_empty)
                .unwrap_or(true)
        {
            return Err(Error::new(
                field.span(),
                "Setting aliases without setting a long-switch is an error, \
                make one of the aliases the primary switch name.",
            ));
        }

        if result.env_name.is_none()
            && !result
                .env_aliases
                .as_ref()
                .map(LitStrArray::is_empty)
                .unwrap_or(true)
        {
            return Err(Error::new(
                field.span(),
                "Setting env_aliases without setting an env is an error, \
                make one of the aliases the primary env.",
            ));
        }

        Ok(result)
    }

    pub fn get_field_name(&self) -> &Ident {
        &self.field_name
    }

    pub fn get_field_type(&self) -> Type {
        self.field_type.clone()
    }

    pub fn get_serde_name(&self) -> LitStr {
        self.serde
            .as_ref()
            .and_then(|serde| serde.rename.clone())
            .unwrap_or_else(|| LitStr::new(&self.field_name.to_string(), self.field_name.span()))
    }

    pub fn get_serde_aliases(&self) -> Vec<LitStr> {
        self.serde
            .as_ref()
            .map(|serde| serde.aliases.clone())
            .unwrap_or_default()
    }

    pub fn get_serde_type(&self) -> Type {
        // Check for try_from first
        if let Some(try_from_type) = self.serde.as_ref().and_then(|serde| serde.try_from.clone()) {
            // For Vec<T> fields with try_from = "U", deserialize Vec<U>
            // and convert each element via TryFrom
            return parse_quote! { ::std::vec::Vec<#try_from_type> };
        }

        let use_value_parser = self
            .serde
            .as_ref()
            .map(|serde| serde.use_value_parser)
            .unwrap_or(false);

        if use_value_parser {
            parse_quote! { ::std::vec::Vec<::std::string::String> }
        } else {
            self.field_type.clone()
        }
    }

    pub fn get_serde_try_from(&self) -> Option<Type> {
        self.serde.as_ref().and_then(|serde| serde.try_from.clone())
    }

    pub fn get_serde_skip(&self) -> bool {
        self.serde.as_ref().map(|serde| serde.skip).unwrap_or(false)
    }

    pub fn get_serde_deserialize_with(&self) -> Option<Path> {
        self.serde
            .as_ref()
            .and_then(|serde| serde.deserialize_with.clone())
    }

    /// Returns true if this field can receive a value from serde deserialization
    pub fn has_serde_source(&self) -> bool {
        self.serde.is_some() && !self.get_serde_skip()
    }

    /// Returns true if this field has a CLI or env source (registered with clap)
    pub fn has_cli_or_env_source(&self) -> bool {
        self.short_switch.is_some()
            || self.long_switch.is_some()
            || self.env_name.is_some()
            || self.is_positional
    }

    /// Generate a routine that pushes a ::conf::ProgramOption corresponding to
    /// this field, onto a mut Vec<ProgramOption> that is in scope.
    ///
    pub fn gen_program_option_node(&self) -> Result<Option<TokenStream>, Error> {
        let id = self.field_name.to_string();
        let description = quote_opt_cow(&self.description);
        let short_form = quote_opt(&self.short_switch);
        let long_form = quote_opt_cow(&self.long_switch);
        let env_form = quote_opt_cow(&self.env_name);
        let allow_hyphen_values = self.allow_hyphen_values;
        let secret = quote_opt(&self.secret);
        let is_positional = self.is_positional;
        let has_serde_source = self.has_serde_source();

        let aliases = self
            .aliases
            .as_ref()
            .map(|x| x.quote_elements_cow())
            .unwrap_or_default();
        let env_aliases = self
            .env_aliases
            .as_ref()
            .map(|x| x.quote_elements_cow())
            .unwrap_or_default();

        Ok(Some(quote! {
            ::conf::lazybuf::Node::Leaf(::conf::ProgramOption {
              id: ::std::borrow::Cow::Borrowed(#id),
              parse_type: ::conf::ParseType::Repeat,
              description: #description,
              short_form: #short_form,
              long_form: #long_form,
              aliases: ::std::borrow::Cow::Borrowed(&[#aliases]),
              env_form: #env_form,
              env_aliases: ::std::borrow::Cow::Borrowed(&[#env_aliases]),
              default_help_str: None,
              is_required: false,
              allow_hyphen_values: #allow_hyphen_values,
              secret: #secret,
              is_positional: #is_positional,
              has_serde_source: #has_serde_source,
            })
        }))
    }

    pub fn gen_push_subcommands(
        &self,
        _subcommands_ident: &Ident,
        _parsed_env: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        Ok(quote! {})
    }

    fn get_delimiter(&self) -> TokenStream {
        quote_opt(&if self.no_env_delimiter {
            None
        } else {
            // Default delimiter is comma for both value_parser and value_parser_os
            Some(
                self.env_delimiter
                    .clone()
                    .unwrap_or_else(|| LitChar::new(',', self.field_name.span())),
            )
        })
    }

    fn get_value_parser_expr(&self) -> ValueParserExpr {
        // If we have an explicit OsStr parser, use it
        if let Some(ref value_parser_os) = self.value_parser_os {
            return ValueParserExpr::OsStr(value_parser_os.clone());
        }

        // If we have an explicit value parser, use it
        if let Some(ref value_parser) = self.value_parser {
            return ValueParserExpr::Str(value_parser.clone());
        }

        // Auto-detect Vec<PathBuf> and Vec<OsString> and provide default parsers
        // This only happens when no explicit parser is specified
        // Note: field_type is Vec<T>, so we need to extract T first
        if let Ok(Some(inner_type)) = type_is_vec(&self.field_type) {
            if type_is_pathbuf(&inner_type) {
                return ValueParserExpr::OsStr(
                    parse_quote! { |s: &::std::ffi::OsStr| -> Result<::std::path::PathBuf, ::std::convert::Infallible> { Ok(s.into()) } },
                );
            }

            if type_is_osstring(&inner_type) {
                return ValueParserExpr::OsStr(
                    parse_quote! { |s: &::std::ffi::OsStr| -> Result<::std::ffi::OsString, ::std::convert::Infallible> { Ok(s.into()) } },
                );
            }
        }

        // Default is FromStr::from_str which takes &str
        ValueParserExpr::Str(parse_quote! { std::str::FromStr::from_str })
    }

    /// Generate initializer code for this repeat field.
    ///
    /// # `if_no_conf_context_val` callback
    ///
    /// Callback invoked when conf_context doesn't find a value (when `maybe_val` is `None`).
    /// For repeat fields without serde, this returns an empty vec.
    /// For repeat fields with serde, this returns the document value (with appropriate conversions).
    fn gen_initializer_helper(
        &self,
        conf_context_ident: &Ident,
        if_no_conf_context_val: &dyn Fn() -> TokenStream,
    ) -> Result<(TokenStream, bool), syn::Error> {
        let field_type = &self.field_type;
        let id = self.field_name.to_string();

        let delimiter = self.get_delimiter();
        let value_parser_expr = self.get_value_parser_expr();
        let if_no_conf_context_val = (if_no_conf_context_val)();

        // Note: We can't use rust into_iter, collect, map_err because sometimes it messes with type
        // inference around the value parser
        //
        // Note: The line `let mut result: #field_type = Default::default();`
        // is expected to be default initializing a Vec.
        // It can't be `#field_type::default()` because it requires turbofish syntax.
        //
        // If it fails because the user put another funky type there, imo this should not really be
        // supported. It's more compelling to make the value_parser option easier to use
        // (easier type inference) than to support user-defined containers here, and try to
        // use `.collect` etc. directly into their container. The user's code can do
        // .iter().collect() after our code runs if they want.

        let initializer = match value_parser_expr {
            ValueParserExpr::OsStr(value_parser_os) => {
                // OsStr-based value parser
                quote! {
                  {
                    fn __value_parser__(
                      __arg__: &::std::ffi::OsStr
                    ) -> Result<<#field_type as ::conf::InnerTypeHelper>::Ty, impl ::core::fmt::Display> {
                      (#value_parser_os)(__arg__)
                    }

                    use ::conf::{ConfValueSource, ProgramOption, InnerError};
                    use ::std::vec::Vec;
                    use ::std::ffi::OsStr;

                    let (maybe_val, opt): (Option<_>, &ProgramOption)
                      = #conf_context_ident.get_repeat_osstring_opt(#id, #delimiter).map_err(|err| vec![err])?;
                    debug_assert!(
                        maybe_val.as_ref().map_or(true, |(vs, _)| !vs.is_default()),
                        "ConfContext should never return Default - the proc-macro generates default logic"
                    );

                    let (value_source, strs): (ConfValueSource<&str>, Vec<&OsStr>) = if let Some(val) = maybe_val {
                        val
                    } else {
                        #if_no_conf_context_val
                    };

                    #conf_context_ident.log_config_event(#id, value_source);

                    let mut result: #field_type = Default::default();
                    let mut errors = Vec::<InnerError>::new();
                    result.reserve(strs.len());
                    for val_os_str in strs {
                      // Note: val_os_str is already &OsStr, no conversion needed
                      match __value_parser__(val_os_str) {
                        Ok(val) => result.push(val),
                        Err(err) => errors.push(
                          InnerError::invalid_value_os(
                            value_source.clone(),
                            val_os_str,
                            opt,
                            err.to_string()
                          )
                        ),
                      }
                    }
                    if errors.is_empty() {
                      Ok(result)
                    } else {
                      Err(errors)
                    }
                  }
                }
            }
            ValueParserExpr::Str(value_parser) => {
                // String-based value parser - use get_repeat_opt
                quote! {
                  {
                    fn __value_parser__(
                      __arg__: &str
                    ) -> Result<<#field_type as ::conf::InnerTypeHelper>::Ty, impl ::core::fmt::Display> {
                      (#value_parser)(__arg__)
                    }

                    use ::conf::{ConfValueSource, ProgramOption, InnerError};
                    use ::std::vec::Vec;

                    let (maybe_val, opt): (Option<_>, &ProgramOption)
                      = #conf_context_ident.get_repeat_opt(#id, #delimiter).map_err(|err| vec![err])?;
                    debug_assert!(
                        maybe_val.as_ref().map_or(true, |(vs, _)| !vs.is_default()),
                        "ConfContext should never return Default - the proc-macro generates default logic"
                    );

                    let (value_source, strs): (ConfValueSource<&str>, Vec<&str>) = if let Some(val) = maybe_val {
                        val
                    } else {
                        #if_no_conf_context_val
                    };

                    #conf_context_ident.log_config_event(#id, value_source);

                    let mut result: #field_type = Default::default();
                    let mut errors = Vec::<InnerError>::new();
                    result.reserve(strs.len());
                    for val_str in strs {
                      match __value_parser__(val_str) {
                        Ok(val) => result.push(val),
                        Err(err) => errors.push(
                          InnerError::invalid_value(
                            value_source.clone(),
                            val_str,
                            opt,
                            err.to_string()
                          )
                        ),
                      }
                    }
                    if errors.is_empty() {
                      Ok(result)
                    } else {
                      Err(errors)
                    }
                  }
                }
            }
        };
        Ok((initializer, true))
    }

    // Gen initializer
    //
    // Create an expression that returns initialized #field_type value, or errors.
    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        // If there's no CLI or env source, this repeat wasn't registered with clap,
        // so we just return the default value (empty Vec)
        if !self.has_cli_or_env_source() {
            let field_type = &self.field_type;
            return Ok((
                quote! {
                    {
                        let _ = #conf_context_ident;
                        let result: #field_type = Default::default();
                        Ok(result)
                    }
                },
                false,
            ));
        }

        let id = self.field_name.to_string();
        let if_no_conf_context_val = || {
            quote! {
                // No args/env found, return empty vec (repeat fields default to empty)
                #conf_context_ident.log_config_event(#id, ::conf::ConfValueSource::Default);
                return Ok(Default::default());
            }
        };
        self.gen_initializer_helper(conf_context_ident, &if_no_conf_context_val)
    }

    // Gen initializer with a provided doc val
    //
    // This should work similar to gen_initializer, but if the conf context produces a default
    // value, we should return the doc_val instead because it has higher priority than default,
    // but lower than args and env.
    pub fn gen_initializer_with_doc_val(
        &self,
        conf_context_ident: &Ident,
        doc_name: &Ident,
        doc_val: &Ident,
    ) -> Result<(TokenStream, bool), Error> {
        // If there's no CLI or env source, this repeat wasn't registered with clap,
        // so we just use the serde value directly
        if !self.has_cli_or_env_source() {
            let id = self.field_name.to_string();
            let try_from = self.get_serde_try_from();

            if let Some(_try_from_type) = try_from {
                // Convert each element via TryFrom
                let field_name_str = self.field_name.to_string();
                let inner_type = type_is_vec(&self.field_type)?
                    .ok_or_else(|| Error::new(self.field_type.span(), "Expected Vec<T> type"))?;

                return Ok((
                    quote! {
                        {
                            #conf_context_ident.log_config_event(
                                #id,
                                ::conf::ConfValueSource::Document(#doc_name)
                            );
                            let mut __result__ = Vec::with_capacity(#doc_val.len());
                            for __item__ in #doc_val {
                                match <#inner_type as ::core::convert::TryFrom<_>>::try_from(__item__) {
                                    Ok(__converted__) => __result__.push(__converted__),
                                    Err(__err__) => {
                                        return Err(vec![::conf::InnerError::serde(
                                            #doc_name,
                                            #field_name_str,
                                            __err__
                                        )]);
                                    }
                                }
                            }
                            Ok(__result__)
                        }
                    },
                    false,
                ));
            } else {
                return Ok((
                    quote! {
                        {
                            #conf_context_ident.log_config_event(
                                #id,
                                ::conf::ConfValueSource::Document(#doc_name)
                            );
                            Ok(#doc_val)
                        }
                    },
                    false,
                ));
            }
        }

        let use_value_parser = self
            .serde
            .as_ref()
            .map(|serde| serde.use_value_parser)
            .unwrap_or(false);

        let try_from = self.get_serde_try_from();

        if let Some(_try_from_type) = try_from {
            // When try_from is set, #doc_val has type Vec<try_from_type>.
            // We convert each element via TryFrom to get Vec<inner_type>.
            let field_name_str = self.field_name.to_string();

            // Get the inner type of Vec<T>
            let inner_type = type_is_vec(&self.field_type)?
                .ok_or_else(|| Error::new(self.field_type.span(), "Expected Vec<T> type"))?;

            let id = self.field_name.to_string();
            let if_no_conf_context_val = || {
                quote! {
                    #conf_context_ident.log_config_event(
                        #id,
                        ::conf::ConfValueSource::Document(#doc_name)
                    );
                    let mut __result__ = Vec::with_capacity(#doc_val.len());
                    for __item__ in #doc_val {
                        match <#inner_type as ::core::convert::TryFrom<_>>::try_from(__item__) {
                            Ok(__converted__) => __result__.push(__converted__),
                            Err(__err__) => {
                                return Err(vec![::conf::InnerError::serde(
                                    #doc_name,
                                    #field_name_str,
                                    __err__
                                )]);
                            }
                        }
                    }
                    return Ok(__result__);
                }
            };

            self.gen_initializer_helper(conf_context_ident, &if_no_conf_context_val)
        } else if use_value_parser {
            // When use_value_parser is enabled and no args/env found, use document value.
            // The value_parser will be run on each element from the document.
            let value_parser_expr = self.get_value_parser_expr();
            let if_no_conf_context_val = || match value_parser_expr {
                ValueParserExpr::Str(_) => quote! {
                    (ConfValueSource::Document(#doc_name), #doc_val.iter().map(String::as_str).collect())
                },
                ValueParserExpr::OsStr(_) => quote! {
                    (ConfValueSource::Document(#doc_name), #doc_val.iter().map(|s| ::std::ffi::OsStr::new(s.as_str())).collect())
                },
            };

            self.gen_initializer_helper(conf_context_ident, &if_no_conf_context_val)
        } else {
            // When use_value_parser is not enabled and no args/env found, return doc value directly.
            let id = self.field_name.to_string();
            let if_no_conf_context_val = || {
                quote! {
                    #conf_context_ident.log_config_event(
                        #id,
                        ::conf::ConfValueSource::Document(#doc_name)
                    );
                    return Ok(#doc_val);
                }
            };

            self.gen_initializer_helper(conf_context_ident, &if_no_conf_context_val)
        }
    }

    /// Generate debug assertions for this repeat
    /// Repeats don't support default_value, so nothing to check
    pub fn gen_debug_asserts(&self, _struct_ident: &Ident) -> Result<TokenStream, Error> {
        Ok(quote! {})
    }
}
