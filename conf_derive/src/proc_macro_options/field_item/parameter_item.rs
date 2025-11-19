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
    pub use_value_parser: bool,
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
        if result.value_parser.is_some() && result.value_parser_os.is_some() {
            return Err(Error::new(
                field.span(),
                "#[conf(value_parser)] and #[conf(value_parser_os)] cannot both be specified",
            ));
        }

        if result.is_optional_type.is_none()
            && result.short_switch.is_none()
            && result.long_switch.is_none()
            && result.env_name.is_none()
            && result.default_value.is_none()
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
            .map(|serde| serde.use_value_parser)
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

    pub fn gen_push_program_options(
        &self,
        program_options_ident: &Ident,
    ) -> Result<TokenStream, syn::Error> {
        let is_required = self.is_optional_type.is_none() && self.default_value.is_none();
        let id = self.field_name.to_string();
        let description = quote_opt_into(&self.doc_string);
        let short_form = quote_opt(&self.short_switch);
        let long_form = quote_opt_into(&self.long_switch);
        let aliases = self.aliases.as_ref().map(LitStrArray::quote_elements_into);
        let env_form = quote_opt_into(&self.env_name);
        let env_aliases = self
            .env_aliases
            .as_ref()
            .map(LitStrArray::quote_elements_into);
        let default_value = quote_opt_into(&self.default_value);
        let allow_hyphen_values = self.allow_hyphen_values;
        let secret = quote_opt(&self.secret);
        let is_positional = self.is_positional;
        let has_serde_source = self.has_serde_source();

        Ok(quote! {
            #program_options_ident.push(::conf::ProgramOption {
                id: #id.into(),
                parse_type: ::conf::ParseType::Parameter,
                description: #description,
                short_form: #short_form,
                long_form: #long_form,
                aliases: vec![#aliases],
                env_form: #env_form,
                env_aliases: vec![#env_aliases],
                default_value: #default_value,
                is_required: #is_required,
                allow_hyphen_values: #allow_hyphen_values,
                secret: #secret,
                is_positional: #is_positional,
                has_serde_source: #has_serde_source,
            });
        })
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
    ///
    /// # `before_value_parser` callback
    ///
    /// If provided, `before_value_parser` is a function that takes an `ExprRequest` and returns
    /// a `TokenStream`. It will be invoked with the appropriate request type based on the value parser.
    ///
    /// The generated code creates these variables before calling `before_value_parser`:
    /// - `value_source: ConfValueSource<&str>` - where the value came from
    /// - `val_str: &str` (for `ExprRequest::Str`) or `val_os: &OsStr` (for `ExprRequest::OsStr`)
    /// - `opt: &ProgramOption` - the option metadata
    ///
    /// The `before_value_parser` TokenStream can:
    /// - Reference these variables
    /// - Shadow them with new values (e.g., replacing with serde document value)
    /// - Early return from the function if needed
    ///
    /// After `before_value_parser` executes, the code expects:
    /// - `value_source` and `val_str`/`val_os` to be in scope (possibly shadowed)
    /// - The variable to have the same type as initially created
    fn gen_initializer_helper(
        &self,
        conf_context_ident: &Ident,
        if_no_conf_context_val: &dyn Fn(ExprRequest) -> TokenStream,
        before_value_parser: Option<&dyn Fn(ExprRequest) -> TokenStream>,
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
                let before_value_parser = before_value_parser.map(|f| (f)(ExprRequest::OsStr));
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
                    let (value_source, val_os): (ConfValueSource<&str>, &::std::ffi::OsStr)
                      = if let Some(val) = maybe_val {
                        val
                      } else {
                        #if_no_conf_context_val
                      };
                    #before_value_parser
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
                let before_value_parser = before_value_parser.map(|f| (f)(ExprRequest::Str));
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
                    let (value_source, val_str): (ConfValueSource<&str>, &str)
                      = if let Some(val) = maybe_val {
                        val
                      } else {
                        #if_no_conf_context_val
                      };
                    #before_value_parser
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
        // Default behavior when no conf context value is found
        let if_no_conf_context_val = |_req: ExprRequest| {
            if self.is_optional_type.is_some() {
                quote! { return Ok(None); }
            } else {
                quote! { return Err(#conf_context_ident.missing_required_parameter_error(opt)); }
            }
        };
        self.gen_initializer_helper(conf_context_ident, &if_no_conf_context_val, None)
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
        let use_value_parser = self
            .serde
            .as_ref()
            .map(|serde| serde.use_value_parser)
            .unwrap_or(false);

        let try_from = self.get_serde_try_from();

        if let Some(_try_from_type) = try_from {
            // When try_from is set, #doc_val has the try_from type (or Option<try_from_type> for optional fields).
            // To pick this value for the field, we use TryFrom::try_from to convert.
            let field_type = &self.field_type;
            let field_name_str = self.field_name.to_string();

            // For Option<T> fields, we deserialize Option<U> and map the conversion
            // For non-optional fields, we convert directly
            if let Some(inner_type) = &self.is_optional_type {
                let if_no_conf_context_val = |_| {
                    quote! {
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

                let before_value_parser = |_| {
                    quote! {
                        if value_source.is_default() {
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
                    }
                };
                self.gen_initializer_helper(
                    conf_context_ident,
                    &if_no_conf_context_val,
                    Some(&before_value_parser),
                )
            } else {
                let if_no_conf_context_val = |_| {
                    quote! {
                        return <#field_type as ::core::convert::TryFrom<_>>::try_from(#doc_val)
                            .map_err(|err| ::conf::InnerError::serde(
                                #doc_name,
                                #field_name_str,
                                err
                            ));
                    }
                };

                let before_value_parser = |_| {
                    quote! {
                        if value_source.is_default() {
                            return <#field_type as ::core::convert::TryFrom<_>>::try_from(#doc_val)
                                .map_err(|err| ::conf::InnerError::serde(
                                    #doc_name,
                                    #field_name_str,
                                    err
                                ));
                        }
                    }
                };
                self.gen_initializer_helper(
                    conf_context_ident,
                    &if_no_conf_context_val,
                    Some(&before_value_parser),
                )
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
            let before_value_parser = |req: ExprRequest| -> TokenStream {
                match req {
                    ExprRequest::Str => quote! {
                      let (value_source, val_str) = if value_source.is_default() {
                        (ConfValueSource::Document(#doc_name), #doc_val.as_str())
                      } else {
                        (value_source, val_str)
                      };
                    },
                    ExprRequest::OsStr => quote! {
                      let (value_source, val_os) = if value_source.is_default() {
                        (ConfValueSource::Document(#doc_name), ::std::ffi::OsStr::new(#doc_val.as_str()))
                      } else {
                        (value_source, val_os)
                      };
                    },
                }
            };
            self.gen_initializer_helper(
                conf_context_ident,
                &if_no_conf_context_val,
                Some(&before_value_parser),
            )
        } else {
            // When use_value_parser is false, then #doc_val has type #field_type.
            // To pick this value for the field, we just return it.
            let if_no_conf_context_val = |_| {
                quote! {
                  return Ok(#doc_val);
                }
            };

            let before_value_parser = |_| {
                quote! {
                  if value_source.is_default() {
                    return Ok(#doc_val);
                  }
                }
            };
            self.gen_initializer_helper(
                conf_context_ident,
                &if_no_conf_context_val,
                Some(&before_value_parser),
            )
        }
    }

    /// Generate debug assertions for this parameter
    /// If there's a default_value and a value_parser/value_parser_os, test that the default parses
    pub fn gen_debug_asserts(&self, struct_ident: &Ident) -> Result<TokenStream, Error> {
        // Check if skip_default_value is set. This might be useful when the value_parser has
        // side-effects or reads files from the file-system that might not be there during the test.
        if let Some(test_item) = &self.test {
            if test_item.skip_default_value {
                return Ok(quote! {});
            }
        }

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

            Ok(parse_expr)
        } else {
            Ok(quote! {})
        }
    }
}
