use super::{ExprRequest, StructItem, ValueParserExpr};
use crate::util::*;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Error, Expr, Field, Ident, LitBool, LitChar, LitStr, Path, Type, meta::ParseNestedMeta,
    parse_quote, spanned::Spanned, token,
};

/// #[conf(serde(...))] options listed on a parameter
pub struct ParameterSerdeItem {
    pub rename: Option<LitStr>,
    pub aliases: Vec<LitStr>,
    pub skip: bool,
    pub use_value_parser: Option<Span>,
    pub deserialize_with: Option<Path>,
    pub try_from: Option<Type>,
    span: Span,
}

impl ParameterSerdeItem {
    pub fn new(meta: ParseNestedMeta<'_>) -> Result<Self, Error> {
        let mut result = Self {
            rename: None,
            aliases: Vec::new(),
            skip: false,
            use_value_parser: None,
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
                    set_once(&path, &mut result.use_value_parser, Some(path.span()))
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
        if let (Some(deserialize_with), Some(use_value_parser)) =
            (&result.deserialize_with, &result.use_value_parser)
        {
            return Err(mutually_exclusive_error(
                "deserialize_with",
                deserialize_with,
                "use_value_parser",
                use_value_parser,
            ));
        }

        if let (Some(try_from), Some(use_value_parser)) =
            (&result.try_from, &result.use_value_parser)
        {
            return Err(mutually_exclusive_error(
                "try_from",
                try_from,
                "use_value_parser",
                use_value_parser,
            ));
        }

        if let (Some(try_from), Some(deserialize_with)) =
            (&result.try_from, &result.deserialize_with)
        {
            return Err(mutually_exclusive_error(
                "try_from",
                try_from,
                "deserialize_with",
                deserialize_with,
            ));
        }

        Ok(result)
    }
}

/// #[conf(test(...))] options listed on a parameter
pub struct ParameterTestItem {
    pub skip_default_value: bool,
    span: Span,
}

impl ParameterTestItem {
    pub fn new(meta: ParseNestedMeta<'_>) -> Result<Self, Error> {
        let mut result = Self {
            skip_default_value: false,
            span: meta.input.span(),
        };

        if meta.input.peek(token::Paren) {
            meta.parse_nested_meta(|meta| {
                let path = meta.path.clone();
                if path.is_ident("skip_default_value") {
                    result.skip_default_value = true;
                    Ok(())
                } else {
                    Err(meta.error("unrecognized conf(test) option"))
                }
            })?;
        }

        Ok(result)
    }
}

impl GetSpan for ParameterTestItem {
    fn get_span(&self) -> Span {
        self.span
    }
}

impl GetSpan for ParameterSerdeItem {
    fn get_span(&self) -> Span {
        self.span
    }
}

/// Proc macro annotations parsed from a field of Parameter kind
pub struct ParameterItem {
    field_name: Ident,
    field_type: Type,
    is_optional_type: Option<Type>,
    allow_hyphen_values: bool,
    secret: Option<LitBool>,
    short_switch: Option<LitChar>,
    long_switch: Option<LitStr>,
    aliases: Option<LitStrArray>,
    env_name: Option<LitStr>,
    env_aliases: Option<LitStrArray>,
    default_value: Option<LitStr>,
    default_value_expr: Option<Expr>,
    default_help_str: Option<LitStr>,
    value_parser: Option<Expr>,
    value_parser_os: Option<Expr>,
    serde: Option<ParameterSerdeItem>,
    test: Option<ParameterTestItem>,
    doc_string: Option<String>,
    is_positional: bool,
}

impl ParameterItem {
    pub fn new(field: &Field, struct_item: &StructItem) -> Result<Self, Error> {
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "missing identifier"))?;
        let field_type = field.ty.clone();
        let is_optional_type = type_is_option(&field.ty)?;
        // signed numbers often start with hyphens
        let allow_hyphen_values = type_is_signed_number(&field.ty);

        let mut result = Self {
            field_name,
            field_type,
            is_optional_type,
            allow_hyphen_values,
            secret: None,
            short_switch: None,
            long_switch: None,
            aliases: None,
            env_name: None,
            env_aliases: None,
            default_value: None,
            default_value_expr: None,
            default_help_str: None,
            value_parser: None,
            value_parser_os: None,
            serde: None,
            test: None,
            doc_string: None,
            is_positional: false,
        };

        for attr in &field.attrs {
            maybe_append_doc_string(&mut result.doc_string, &attr.meta)?;
            if attr.path().is_ident("conf") || attr.path().is_ident("arg") {
                attr.parse_nested_meta(|meta| {
                    let path = meta.path.clone();
                    if path.is_ident("parameter") {
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
                    } else if path.is_ident("default_value") {
                        let val = meta.value()?.parse::<LitStr>()?;
                        set_once(&path, &mut result.default_value, Some(val))
                    } else if path.is_ident("default_help_str") {
                        let val = meta.value()?.parse::<LitStr>()?;
                        set_once(&path, &mut result.default_help_str, Some(val))
                    } else if path.is_ident("default") {
                        let expr = if meta.input.peek(token::Paren) {
                            // default(<expr>)
                            let content;
                            syn::parenthesized!(content in meta.input);
                            content.parse::<Expr>()?
                        } else {
                            // default (no parens) => Default::default()
                            parse_quote! { Default::default() }
                        };
                        set_once(&path, &mut result.default_value_expr, Some(expr))
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
                        set_once(
                            &path,
                            &mut result.serde,
                            Some(ParameterSerdeItem::new(meta)?),
                        )
                    } else if path.is_ident("test") {
                        set_once(&path, &mut result.test, Some(ParameterTestItem::new(meta)?))
                    } else if path.is_ident("pos") {
                        result.is_positional = true;
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized conf parameter option"))
                    }
                })?;
            }
        }

        // Validate positional argument constraints
        if result.is_positional {
            if result.short_switch.is_some() {
                return Err(Error::new(
                    field.span(),
                    "#[conf(pos)] cannot be used with #[conf(short)]",
                ));
            }
            if result.long_switch.is_some() {
                return Err(Error::new(
                    field.span(),
                    "#[conf(pos)] cannot be used with #[conf(long)]",
                ));
            }
        }

        // Validate value_parser and value_parser_os aren't both specified
        if let (Some(value_parser), Some(value_parser_os)) =
            (&result.value_parser, &result.value_parser_os)
        {
            return Err(mutually_exclusive_error(
                "value_parser",
                value_parser,
                "value_parser_os",
                value_parser_os,
            ));
        }

        // Validate default_value and default_value_expr aren't both specified
        if result.default_value.is_some() && result.default_value_expr.is_some() {
            return Err(Error::new(
                field.span(),
                "#[conf(default_value)] and #[conf(default)] cannot both be specified",
            ));
        }

        // Validate default_help_str without default_value or default_value_expr
        if result.default_help_str.is_some()
            && result.default_value.is_none()
            && result.default_value_expr.is_none()
            && result.is_optional_type.is_none()
        {
            return Err(Error::new(
                field.span(),
                "default_help_str is provided but there is no default that it is documenting",
            ));
        }

        // Note: If default_value_expr is used without default_help_str, we generate code that
        // calls Display::fmt on the default value. If the type doesn't implement Display,
        // this will fail at compile time with a clear error message from rustc.

        if result.is_optional_type.is_none()
            && result.short_switch.is_none()
            && result.long_switch.is_none()
            && result.env_name.is_none()
            && result.default_value.is_none()
            && result.default_value_expr.is_none()
            && !result.is_positional
            && struct_item.serde.is_none()
        {
            return Err(Error::new(
                field.span(),
                "There is no way for the user to give this parameter a value. \
                Trying using #[arg(short)], #[arg(long)], #[arg(env)], or #[arg(pos)] to specify a switch, \
                positional argument, or an env associated to this value, or specify a default value.",
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

    pub fn get_default_value(&self) -> Option<&LitStr> {
        self.default_value.as_ref()
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
            // If field is Option<T>, wrap try_from type in Option as well
            // This allows the field to be optional in the JSON
            if self.is_optional_type.is_some() {
                return parse_quote! { ::core::option::Option<#try_from_type> };
            }
            return try_from_type;
        }

        let use_value_parser = self
            .serde
            .as_ref()
            .map(|serde| serde.use_value_parser.is_some())
            .unwrap_or(false);

        if use_value_parser {
            parse_quote! { ::std::string::String }
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

    /// Returns true if this field has a CLI or env source (not just serde)
    pub fn has_cli_or_env_source(&self) -> bool {
        self.short_switch.is_some()
            || self.long_switch.is_some()
            || self.env_name.is_some()
            || self.is_positional
    }

    pub fn gen_program_option_node(&self) -> Result<Option<TokenStream>, Error> {
        let is_required = self.is_optional_type.is_none()
            && self.default_value.is_none()
            && self.default_value_expr.is_none();
        let id = self.field_name.to_string();
        let description = quote_opt_cow(&self.doc_string);
        let short_form = quote_opt(&self.short_switch);
        let long_form = quote_opt_cow(&self.long_switch);
        let env_form = quote_opt_cow(&self.env_name);

        // Generate function pointer for default help text
        let default_help_str = if let Some(help_str_lit) = &self.default_help_str {
            // Explicit default_help_str provided - generate function that writes the literal
            let help_str = help_str_lit.value();
            quote! {
                Some(::conf::DisplayFn((|f: &mut ::core::fmt::Formatter| f.write_str(#help_str)) as fn(&mut ::core::fmt::Formatter) -> ::core::fmt::Result))
            }
        } else if let Some(default_value_lit) = &self.default_value {
            // default_value provided - generate function that writes the literal
            let default_str = default_value_lit.value();
            quote! {
                Some(::conf::DisplayFn((|f: &mut ::core::fmt::Formatter| f.write_str(#default_str)) as fn(&mut ::core::fmt::Formatter) -> ::core::fmt::Result))
            }
        } else if let Some(default_value_expr) = &self.default_value_expr {
            // default_value_expr provided - generate function that evaluates expr and displays it
            // For Option<T> fields, the default_value_expr produces T, not Option<T>
            let field_type = &self.field_type;
            let inner_type = self.is_optional_type.as_ref().unwrap_or(field_type);
            quote! {
                Some(::conf::DisplayFn((|f: &mut ::core::fmt::Formatter| {
                    fn __default_value__() -> #inner_type {
                        #default_value_expr
                    }
                    ::core::fmt::Display::fmt(&__default_value__(), f)
                }) as fn(&mut ::core::fmt::Formatter) -> ::core::fmt::Result))
            }
        } else {
            quote! { None }
        };

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
                parse_type: ::conf::ParseType::Parameter,
                description: #description,
                short_form: #short_form,
                long_form: #long_form,
                aliases: ::std::borrow::Cow::Borrowed(&[#aliases]),
                env_form: #env_form,
                env_aliases: ::std::borrow::Cow::Borrowed(&[#env_aliases]),
                default_help_str: #default_help_str,
                is_required: #is_required,
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

    fn get_value_parser_expr(&self) -> ValueParserExpr {
        // If we have an explicit OsStr parser, use it
        if let Some(parser) = &self.value_parser_os {
            return ValueParserExpr::OsStr(parser.clone());
        }

        // If we have an explicit value parser, use it
        if let Some(parser) = &self.value_parser {
            return ValueParserExpr::Str(parser.clone());
        }

        // Auto-detect PathBuf and OsString and provide default parsers
        // This only happens when no explicit parser is specified
        use crate::util::{type_is_osstring, type_is_pathbuf};
        let inner_type = self.is_optional_type.as_ref().unwrap_or(&self.field_type);

        if type_is_pathbuf(inner_type) {
            return ValueParserExpr::OsStr(
                parse_quote! { |s: &::std::ffi::OsStr| -> Result<::std::path::PathBuf, ::std::convert::Infallible> { Ok(s.into()) } },
            );
        }

        if type_is_osstring(inner_type) {
            return ValueParserExpr::OsStr(
                parse_quote! { |s: &::std::ffi::OsStr| -> Result<::std::ffi::OsString, ::std::convert::Infallible> { Ok(s.into()) } },
            );
        }

        // Default is FromStr::from_str which takes &str
        ValueParserExpr::Str(parse_quote! { std::str::FromStr::from_str })
    }

    /// Generate initializer code for this parameter field.
    ///
    /// # `if_no_conf_context_val` callback
    ///
    /// Callback invoked when conf_context doesn't find a value (when `maybe_val` is `None`).
    /// Receives an `ExprRequest` to generate type-appropriate code.
    /// For required fields without serde, this typically returns an error.
    /// For optional fields without serde, this returns Ok(None).
    /// When serde with use_value_parser is used, this returns the document value converted appropriately.
    fn gen_initializer_helper(
        &self,
        conf_context_ident: &Ident,
        if_no_conf_context_val: &dyn Fn(ExprRequest) -> TokenStream,
    ) -> Result<(TokenStream, bool), syn::Error> {
        let field_type = &self.field_type;
        let id = self.field_name.to_string();

        // Code gen is slightly different if the field type is Option<T>
        // Inner_type is T in that case, or just field_type otherwise.
        // Value parser will produce inner_type.
        let inner_type = self.is_optional_type.as_ref().unwrap_or(field_type);

        // Value parser produces #inner_type, so we have to massage a success result to #field_type
        let value_parser_ok_arm = if self.is_optional_type.is_some() {
            quote! { Ok(t) => Ok(Some(t)), }
        } else {
            quote! { Ok(t) => Ok(t), }
        };

        // The part around value parser needs to be very simple if we want type inference to work
        // We also stick the user-provided expression inside a function to prevent it from mutating
        // anything in the surrounding scope.
        // But we are reading conf_context_ident from our caller's scope, outside of the
        // user-provided expression

        let initializer = match self.get_value_parser_expr() {
            ValueParserExpr::OsStr(value_parser_expr) => {
                let if_no_conf_context_val = (if_no_conf_context_val)(ExprRequest::OsStr);
                // Use OsStr-based value parser
                quote! {
                  {
                    fn __value_parser__(
                      __arg__: &::std::ffi::OsStr
                    ) -> Result<#inner_type, impl ::core::fmt::Display> {
                      (#value_parser_expr)(__arg__)
                    }

                    use ::conf::{ConfValueSource, ProgramOption, InnerError};

                    let (maybe_val, opt): (Option<_>, &ProgramOption)
                      = #conf_context_ident.get_osstring_opt(#id)?;
                    debug_assert!(
                        maybe_val.as_ref().map_or(true, |(vs, _)| !vs.is_default()),
                        "ConfContext should never return Default - the proc-macro generates default logic"
                    );
                    let (value_source, val_os): (ConfValueSource<&str>, &::std::ffi::OsStr)
                      = if let Some(val) = maybe_val {
                        val
                      } else {
                        #if_no_conf_context_val
                      };
                    #conf_context_ident.log_config_event(#id, value_source);
                    match __value_parser__(val_os) {
                      #value_parser_ok_arm
                      Err(err) => Err(
                        InnerError::invalid_value_os(
                          value_source,
                          val_os,
                          opt,
                          err
                        )
                      ),
                    }
                  }
                }
            }
            ValueParserExpr::Str(value_parser_expr) => {
                let if_no_conf_context_val = (if_no_conf_context_val)(ExprRequest::Str);
                // Use str-based value parser - ConfContext handles UTF-8 conversion
                quote! {
                  {
                    fn __value_parser__(
                      __arg__: &str
                    ) -> Result<#inner_type, impl ::core::fmt::Display> {
                      (#value_parser_expr)(__arg__)
                    }

                    use ::conf::{ConfValueSource, ProgramOption, InnerError};

                    let (maybe_val, opt): (Option<_>, &ProgramOption)
                      = #conf_context_ident.get_string_opt(#id)?;
                    debug_assert!(
                        maybe_val.as_ref().map_or(true, |(vs, _)| !vs.is_default()),
                        "ConfContext should never return Default - the proc-macro generates default logic"
                    );
                    let (value_source, val_str): (ConfValueSource<&str>, &str)
                      = if let Some(val) = maybe_val {
                        val
                      } else {
                        #if_no_conf_context_val
                      };
                    #conf_context_ident.log_config_event(#id, value_source);
                    match __value_parser__(val_str) {
                      #value_parser_ok_arm
                      Err(err) => Err(
                        InnerError::invalid_value(
                          value_source,
                          val_str,
                          opt,
                          err
                        )
                      ),
                    }
                  }
                }
            }
        };
        Ok((initializer, false))
    }

    pub fn gen_initializer(
        &self,
        conf_context_ident: &Ident,
    ) -> Result<(TokenStream, bool), syn::Error> {
        // If there's no CLI or env source, this parameter wasn't registered with clap,
        // so we handle it specially
        if !self.has_cli_or_env_source() {
            // Check for default value first, before checking if optional
            // This ensures that optional fields with defaults get Some(default) not None
            if let Some(default_value) = &self.default_value {
                // Serde-only field with default: use gen_initializer_helper with callbacks
                // that provide the default value instead of reading from conf_context
                let default_value_str = default_value.value();

                let if_no_conf_context_val = |req: ExprRequest| -> TokenStream {
                    match req {
                        ExprRequest::Str => quote! {
                            (::conf::ConfValueSource::Default, #default_value_str)
                        },
                        ExprRequest::OsStr => quote! {
                            (::conf::ConfValueSource::Default, ::std::ffi::OsStr::new(#default_value_str))
                        },
                    }
                };

                return self.gen_initializer_helper(conf_context_ident, &if_no_conf_context_val);
            } else if let Some(default_value_expr) = &self.default_value_expr {
                // Serde-only field with default expression: bypass value parser
                let field_type = &self.field_type;
                let inner_type = self.is_optional_type.as_ref().unwrap_or(field_type);
                let wrap_in_some = if self.is_optional_type.is_some() {
                    quote! { Some(__default_value_result__) }
                } else {
                    quote! { __default_value_result__ }
                };
                let id = self.field_name.to_string();
                return Ok((
                    quote! {
                        {
                            fn __default_value__() -> #inner_type {
                                #default_value_expr
                            }
                            let _ = #conf_context_ident;
                            let __default_value_result__ = __default_value__();
                            #conf_context_ident.log_config_event(#id, ::conf::ConfValueSource::Default);
                            Ok(#wrap_in_some)
                        }
                    },
                    false,
                ));
            } else if self.is_optional_type.is_some() {
                // Serde-only optional field without default: return None
                return Ok((
                    quote! {
                        {
                            let _ = #conf_context_ident;
                            Ok(None)
                        }
                    },
                    false,
                ));
            } else {
                // Serde-only required field with no default: this is an error
                // This field can only be satisfied from a serde document, and it's not optional,
                // so if we're here it means either no document was provided or the field was missing
                let id = self.field_name.to_string();
                return Ok((
                    quote! {
                        {
                            // Get the program option for this field to use in the error
                            let opt = #conf_context_ident.get_program_option_by_id(#id)
                                .expect("internal error: program option should exist for this field");
                            Err(#conf_context_ident.missing_required_parameter_error(opt))
                        }
                    },
                    false,
                ));
            }
        }

        // Default behavior when no conf context value is found
        let if_no_conf_context_val = |req: ExprRequest| {
            // Check for default_value_expr first - if present, bypass value parser
            if let Some(default_value_expr) = &self.default_value_expr {
                let field_type = &self.field_type;
                let inner_type = self.is_optional_type.as_ref().unwrap_or(field_type);
                let wrap_in_some = if self.is_optional_type.is_some() {
                    quote! { Some(__default_value_result__) }
                } else {
                    quote! { __default_value_result__ }
                };
                let id = self.field_name.to_string();
                return quote! {
                    {
                        fn __default_value__() -> #inner_type {
                            #default_value_expr
                        }
                        let __default_value_result__ = __default_value__();
                        #conf_context_ident.log_config_event(#id, ::conf::ConfValueSource::Default);
                        return Ok(#wrap_in_some);
                    }
                };
            }
            // Check for default_value (goes through value parser)
            if let Some(default_value) = &self.default_value {
                let default_value_str = default_value.value();
                match req {
                    ExprRequest::Str => quote! {
                        (::conf::ConfValueSource::Default, #default_value_str)
                    },
                    ExprRequest::OsStr => quote! {
                        (::conf::ConfValueSource::Default, ::std::ffi::OsStr::new(#default_value_str))
                    },
                }
            } else if self.is_optional_type.is_some() {
                quote! { return Ok(None); }
            } else {
                quote! { return Err(#conf_context_ident.missing_required_parameter_error(opt)); }
            }
        };
        self.gen_initializer_helper(conf_context_ident, &if_no_conf_context_val)
    }

    // Gen initializer with a provided document value.
    //
    // Like gen_initializer, but in this case, serde has provided a value for this field.
    // The value is a variable of type #serde_type and the identifier is #doc_val.
    //
    // Here, we should return #doc_val if the basic initializer would have produced no value,
    // or would have produced the default_value string, because the document is higher priority.
    // But the document value should be ignored if args or env is the value source.
    pub fn gen_initializer_with_doc_val(
        &self,
        conf_context_ident: &Ident,
        doc_name: &Ident,
        doc_val: &Ident,
    ) -> Result<(TokenStream, bool), Error> {
        // If there's no CLI or env source, this parameter wasn't registered with clap,
        // so we just return the doc value directly
        if !self.has_cli_or_env_source() {
            let id = self.field_name.to_string();
            return Ok((
                quote! {
                    {
                        let _ = #conf_context_ident;
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

        let use_value_parser = self
            .serde
            .as_ref()
            .map(|serde| serde.use_value_parser.is_some())
            .unwrap_or(false);

        let try_from = self.get_serde_try_from();

        if let Some(_try_from_type) = try_from {
            // When try_from is set, #doc_val has the try_from type (or Option<try_from_type> for optional fields).
            // To pick this value for the field, we use TryFrom::try_from to convert.
            let field_type = &self.field_type;
            let id = self.field_name.to_string();
            let field_name_str = self.field_name.to_string();

            // For Option<T> fields, we deserialize Option<U> and map the conversion
            // For non-optional fields, we convert directly
            if let Some(inner_type) = &self.is_optional_type {
                let if_no_conf_context_val = |_| {
                    quote! {
                        #conf_context_ident.log_config_event(
                            #id,
                            ::conf::ConfValueSource::Document(#doc_name)
                        );
                        return match #doc_val {
                            Some(__intermediate__) => {
                                <#inner_type as ::core::convert::TryFrom<_>>::try_from(__intermediate__)
                                    .map(Some)
                                    .map_err(|err| ::conf::InnerError::serde(
                                        #doc_name,
                                        #field_name_str,
                                        err
                                    ))
                            }
                            None => Ok(None),
                        };
                    }
                };

                self.gen_initializer_helper(conf_context_ident, &if_no_conf_context_val)
            } else {
                let if_no_conf_context_val = |_| {
                    quote! {
                        #conf_context_ident.log_config_event(
                            #id,
                            ::conf::ConfValueSource::Document(#doc_name)
                        );
                        return <#field_type as ::core::convert::TryFrom<_>>::try_from(#doc_val)
                            .map_err(|err| ::conf::InnerError::serde(
                                #doc_name,
                                #field_name_str,
                                err
                            ));
                    }
                };

                self.gen_initializer_helper(conf_context_ident, &if_no_conf_context_val)
            }
        } else if use_value_parser {
            // When use_value_parser is true, then #doc_val has type String.
            // To pick this value for the field, we have to set value_source and val_os/val_str
            // to indicate that we are selecting the document value.
            let if_no_conf_context_val = |req: ExprRequest| -> TokenStream {
                match req {
                    ExprRequest::Str => quote! {
                      (ConfValueSource::Document(#doc_name), #doc_val.as_str())
                    },
                    ExprRequest::OsStr => quote! {
                      (ConfValueSource::Document(#doc_name), ::std::ffi::OsStr::new(#doc_val.as_str()))
                    },
                }
            };
            self.gen_initializer_helper(conf_context_ident, &if_no_conf_context_val)
        } else {
            // When use_value_parser is false, then #doc_val has type #field_type.
            // To pick this value for the field, we just return it.
            let id = self.field_name.to_string();
            let if_no_conf_context_val = |_| {
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

    /// Generate debug assertions for this parameter
    /// If there's a default_value and a value_parser/value_parser_os, test that the default parses
    /// If there's a default_value_expr, test that it evaluates without panicking
    pub fn gen_debug_asserts(&self, struct_ident: &Ident) -> Result<TokenStream, Error> {
        // Check if skip_default_value is set. This might be useful when the value_parser has
        // side-effects or reads files from the file-system that might not be there during the test.
        if let Some(test_item) = &self.test {
            if test_item.skip_default_value {
                return Ok(quote! {});
            }
        }

        let mut assertions = Vec::new();

        if let Some(default_value) = &self.default_value {
            let default_value_str = &default_value.value();
            let field_name = &self.field_name;
            let field_type = &self.field_type;
            let inner_type = self.is_optional_type.as_ref().unwrap_or(field_type);

            let do_panic = quote! {
                panic!("in struct '{}' field '{}': default_value '{}' failed to parse: {}",
                    stringify!(#struct_ident), stringify!(#field_name), #default_value_str, err)
            };

            // Use the existing logic to get the value parser
            let value_parser_expr = self.get_value_parser_expr();

            // Generate the appropriate parse expression based on parser type
            // Use the same pattern as gen_initializer_helper: define a local __value_parser__ function
            let parse_expr = match value_parser_expr {
                ValueParserExpr::OsStr(value_parser_expr) => {
                    quote! {
                        {
                            fn __value_parser__(
                                __arg__: &::std::ffi::OsStr
                            ) -> Result<#inner_type, impl ::core::fmt::Display> {
                                (#value_parser_expr)(__arg__)
                            }

                            use ::std::ffi::OsStr;
                            let os_str = OsStr::new(#default_value_str);
                            if let Err(err) = __value_parser__(os_str) { #do_panic }
                        }
                    }
                }
                ValueParserExpr::Str(value_parser_expr) => {
                    quote! {
                        {
                            fn __value_parser__(
                                __arg__: &str
                            ) -> Result<#inner_type, impl ::core::fmt::Display> {
                                (#value_parser_expr)(__arg__)
                            }

                            if let Err(err) = __value_parser__(#default_value_str) { #do_panic }
                        }
                    }
                }
            };

            assertions.push(parse_expr);
        } else if let Some(default_value_expr) = &self.default_value_expr {
            // Test that the default_value_expr evaluates without panicking and produces the right type
            // For Option<T> fields, the default_value_expr produces T, not Option<T>
            let field_type = &self.field_type;
            let inner_type = self.is_optional_type.as_ref().unwrap_or(field_type);

            assertions.push(quote! {
                {
                    fn __default_value__() -> #inner_type {
                        #default_value_expr
                    }

                    // Evaluate the default expression to ensure it doesn't panic
                    // and type-checks correctly
                    let _ = __default_value__();
                }
            });
        }

        Ok(quote! {
            #(#assertions)*
        })
    }
}
