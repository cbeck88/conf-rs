use heck::{ToKebabCase, ToShoutySnakeCase};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::fmt::Display;
use syn::{
    bracketed, meta::ParseNestedMeta, parenthesized, parse::Parse, punctuated::Punctuated,
    spanned::Spanned, Error, Expr, ExprLit, GenericArgument, Lit, LitChar, LitStr, Meta, Path,
    PathArguments, Token, Type,
};

/// Helper for determining if a type is likely bool
pub fn type_is_bool(ty: &Type) -> bool {
    match ty {
        Type::Path(typepath) => typepath.qself.is_none() && typepath.path.is_ident("bool"),
        _ => false,
    }
}

/// Helper for determining if a type looks like Foo<T> for some identifier Foo
/// Returns the inner type T if so, and an error if there are more generic parameters than this
fn type_is_given_generic(generic: &str, ty: &Type) -> Result<Option<Type>, syn::Error> {
    fn path_is_given_generic(generic: &str, path: &Path) -> Result<Option<Type>, syn::Error> {
        if path.leading_colon.is_none() && path.segments.len() == 1 {
            let first = path.segments.first().unwrap();
            if first.ident != generic {
                return Ok(None);
            }

            // We think this is Option<T>
            // Now figure out if there is one generic parameter as expected, and extract it
            // Errors at this point are fatal
            return match &first.arguments {
                PathArguments::AngleBracketed(generic_args) => {
                    if generic_args.args.len() != 1 {
                        return Err(Error::new(
                            generic_args.span(),
                            format!("Expected {generic}<T> for some type T"),
                        ));
                    }
                    let arg = generic_args.args.first().unwrap();
                    match arg {
                        GenericArgument::Type(t) => Ok(Some(t.clone())),
                        _ => Err(Error::new(
                            generic_args.span(),
                            format!("Expected {generic}<T> for some type T"),
                        )),
                    }
                }
                _ => Err(Error::new(
                    first.arguments.span(),
                    format!("Expected {generic}<T> for some type T"),
                )),
            };
        }
        Ok(None)
    }

    match ty {
        Type::Path(typepath) if typepath.qself.is_none() => {
            path_is_given_generic(generic, &typepath.path)
        }
        _ => Ok(None),
    }
}

/// Helper for determining if a type is likely Option<...>
/// Returns first generic argument type if so, returns None if not or if there are no generic arguments
pub fn type_is_option(ty: &Type) -> Result<Option<Type>, syn::Error> {
    type_is_given_generic("Option", ty)
}

/// Helper for determining if a type is likely Vec<...>
pub fn type_is_vec(ty: &Type) -> Result<Option<Type>, syn::Error> {
    type_is_given_generic("Vec", ty)
}

/// Helper for determining if a type is a signed number type
pub fn type_is_signed_number(ty: &Type) -> bool {
    match ty {
        Type::Path(typepath) if typepath.qself.is_none() => {
            let path = &typepath.path;
            path.is_ident("i8")
                || path.is_ident("i16")
                || path.is_ident("i32")
                || path.is_ident("i64")
                || path.is_ident("i128")
                || path.is_ident("f32")
                || path.is_ident("f64")
        }
        _ => false,
    }
}

/// Helper for reading a required value, which comes after a key, during `.parse_nested_meta`
pub fn parse_required_value<T: Parse>(meta: ParseNestedMeta<'_>) -> Result<T, Error> {
    let t: T = meta.value()?.parse()?;
    Ok(t)
}

/// Helper for reading an optional value, which may come after a key, during `.parse_nested_meta`
pub fn parse_optional_value<T: Parse>(meta: ParseNestedMeta<'_>) -> Result<Option<T>, Error> {
    if meta.input.is_empty() || meta.input.peek(Token![,]) {
        Ok(None)
    } else {
        Ok(Some(parse_required_value::<T>(meta)?))
    }
}

/// Helper for making a default short flag for a field
pub fn make_short(ident: &impl Display, span: Span) -> Option<LitChar> {
    let string = ident.to_string();
    if string.is_empty() {
        return None;
    }
    let first = string.to_lowercase().chars().next().unwrap();
    Some(LitChar::new(first, span))
}

/// Helper for making a default long flag for a field
pub fn make_long(ident: &impl Display, span: Span) -> Option<LitStr> {
    let string = ident.to_string();
    if string.is_empty() {
        return None;
    }
    let kebab = string.to_kebab_case();
    Some(LitStr::new(&kebab, span))
}

/// Helper for making a default env flag for a field
pub fn make_env(ident: &impl Display, span: Span) -> Option<LitStr> {
    let string = ident.to_string();
    if string.is_empty() {
        return None;
    }
    let snake = string.to_shouty_snake_case();
    Some(LitStr::new(&snake, span))
}

/// An internal version of Spanned with a blanket implementation, this lets us put it on our custom types more easily.
pub trait GetSpan {
    fn get_span(&self) -> Span;
}

impl<T: Spanned> GetSpan for T {
    fn get_span(&self) -> Span {
        self.span()
    }
}

/// Helper for setting a parameter that should only be set once, during `.parse_nested_meta`
pub fn set_once<T: GetSpan>(
    context: &Path,
    param: &mut Option<T>,
    val: Option<T>,
) -> Result<(), Error> {
    if let Some(param) = param.as_ref() {
        let mut error = Error::new(
            context.span(),
            format!("{} cannot be specified twice", context.get_ident().unwrap()),
        );
        error.combine(Error::new(param.get_span(), "Earlier specified here"));
        return Err(error);
    }
    *param = val;
    Ok(())
}

/// Helper for appending a doc string attribute to the description string, if it is a doc string attribute.
// Based on code here: https://github.com/cyqsimon/documented/blob/e9a465c9e1666839ea08efbe9ce54480d7ee769f/documented-derive/src/lib.rs#L411
pub fn maybe_append_doc_string(
    description: &mut Option<String>,
    attr_meta: &Meta,
) -> Result<(), Error> {
    let doc_expr = match attr_meta {
        Meta::NameValue(ref name_value) if name_value.path.is_ident("doc") => &name_value.value,
        _ => return Ok(()),
    };

    let lit = match doc_expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => s.value(),
        other => {
            return Err(Error::new(
                other.span(),
                "Doc comment is not a string literal",
            ))
        }
    };

    // Split by any newlines pre-existing in the string.
    // Trim any whitespace around those. Then add the newlines back.
    // Terminate with one newline
    let mut trimmed = lit
        .split('\n')
        .map(|line| line.trim())
        .fold(String::new(), |s, line| s + line + "\n");
    // Pop the extra newline
    trimmed.pop();

    // Add to description
    if let Some(desc) = description.as_mut() {
        desc.push('\n');
        desc.push_str(&trimmed);
    } else {
        // Just store this as the description
        *description = Some(trimmed);
    }
    Ok(())
}

/// Helper for turning values Option<String> (or Option<LitStr>) into Option<&'static str> in code generated by quote! macro
pub fn quote_opt<T: ToTokens>(src: &Option<T>) -> TokenStream {
    if let Some(string) = src.as_ref() {
        quote! { Some(#string) }
    } else {
        quote! { None }
    }
}

/// Helper for turning values Option<String> (or Option<LitStr>) into Option<conf::CowStr> in code generated by quote! macro
pub fn quote_opt_into<T: ToTokens>(src: &Option<T>) -> TokenStream {
    if let Some(string) = src.as_ref() {
        quote! { Some(#string.into()) }
    } else {
        quote! { None }
    }
}

/// Helper for parsing an array of string literals (or char literals etc.)
/// After parsing the brackets are dropped
pub struct Array<T: Parse + ToTokens> {
    pub elements: Punctuated<T, Token![,]>,
}

impl<T: Parse + ToTokens> Parse for Array<T> {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let content;
        bracketed!(content in input);
        Ok(Self {
            elements: Punctuated::parse_separated_nonempty(&content)?,
        })
    }
}

impl<T: Parse + ToTokens> GetSpan for Array<T> {
    fn get_span(&self) -> Span {
        self.elements.span()
    }
}

impl<T: Parse + ToTokens> Array<T> {
    pub fn quote_elements_into(&self) -> TokenStream {
        let elements = self.elements.iter();
        quote! { #(#elements .into()),* }
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

pub type LitStrArray = Array<LitStr>;
pub type LitCharArray = Array<LitChar>;

/// Helper for parsing an parenthesized list, of idents for example
/// After parsing the parentheses are dropped
pub struct List<T: Parse + ToTokens> {
    pub elements: Punctuated<T, Token![,]>,
}

impl<T: Parse + ToTokens> Parse for List<T> {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let content;
        parenthesized!(content in input);
        Ok(Self {
            elements: Punctuated::parse_separated_nonempty(&content)?,
        })
    }
}

impl<T: Parse + ToTokens> GetSpan for List<T> {
    fn get_span(&self) -> Span {
        self.elements.span()
    }
}
