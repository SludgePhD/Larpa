use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Token, bracketed, ext::IdentExt, parse::ParseStream, spanned::Spanned};

use crate::name::{ArgName, ArgNames};

pub(super) struct Attr {
    pub(super) name: syn::Ident,
    pub(super) kind: AttrKind,
}

/// A parsed `#[larpa]` attribute.
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(super) enum AttrKind {
    /// `#[larpa(crate = "path::to::larpa")]`
    Crate(syn::Path),

    /// `#[larpa(version = custom_version!())]`
    Version(syn::Expr),

    /// `#[larpa(version_formatter = "custom_fmt")]`
    VersionFormatter(syn::Path),

    /// `#[larpa(no_generate_tests)]`
    NoGenerateTests,

    /// `#[larpa(license = "SPDX-LICENSE-IDENTIFIER")]`
    License(String),

    /// `#[larpa(no_license)]`
    NoLicense,

    /// `#[larpa(homepage = "…")]`
    Homepage(String),

    /// `#[larpa(no_homepage)]`
    NoHomepage,

    /// `#[larpa(repository = "…")]`
    Repository(String),

    /// `#[larpa(no_repository)]`
    NoRepository,

    /// `#[larpa(name = "my-command")]`
    Name(String),

    /// `#[larpa(name = "--name")]`
    ArgName(ArgNames),

    /// `#[larpa(default = "default value as string")]`
    Default(Option<String>),

    /// `#[larpa(flag)]`
    Flag,

    /// `#[larpa(subcommand)]`
    Subcommand,

    /// `#[larpa(required)]`
    Required,

    /// `#[larpa(fallback)]`
    Fallback,

    /// `#[larpa(discover = "discovery_function")]`
    Discover(Option<syn::Path>),

    /// `#[larpa(inverse_of = "field")]`
    InverseOf(syn::Ident),
}

#[derive(Default)]
pub(crate) struct Context {
    krate: Option<syn::Path>,
    refs: Vec<AttrRef>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Target {
    TopLevel,
    Variant,
    Field,
}

impl Context {
    fn parse(&mut self, input: ParseStream<'_>, target: Target) -> syn::Result<Attr> {
        let key = input.call(syn::Ident::parse_any)?;
        self.refs.push(AttrRef {
            target,
            name: key.clone(),
        });

        let kind = match &*key.to_string() {
            // Item-level attributes.
            "crate" => {
                input.parse::<Token![=]>().map_err(|_| {
                    syn::Error::new(key.span(), format!("`{key}` attribute requires a value"))
                })?;
                let string = input.parse::<syn::LitStr>()?;
                let path: syn::Path = string.parse()?;

                self.krate = Some(path.clone());
                AttrKind::Crate(path)
            }
            "version" => {
                input.parse::<Token![=]>().map_err(|_| {
                    syn::Error::new(key.span(), format!("`{key}` attribute requires a value"))
                })?;
                let version = input.parse::<syn::Expr>()?;
                AttrKind::Version(version)
            }
            "version_formatter" => {
                input.parse::<Token![=]>().map_err(|_| {
                    syn::Error::new(key.span(), format!("`{key}` attribute requires a value"))
                })?;
                let string = input.parse::<syn::LitStr>()?;
                let path: syn::Path = string.parse()?;
                AttrKind::VersionFormatter(path)
            }
            "no_generate_tests" => AttrKind::NoGenerateTests,
            "license" => {
                input.parse::<Token![=]>().map_err(|_| {
                    syn::Error::new(key.span(), format!("`{key}` attribute requires a value"))
                })?;
                let string = input.parse::<syn::LitStr>()?;
                AttrKind::License(string.value())
            }
            "no_license" => AttrKind::NoLicense,
            "homepage" => {
                input.parse::<Token![=]>().map_err(|_| {
                    syn::Error::new(key.span(), format!("`{key}` attribute requires a value"))
                })?;
                let string = input.parse::<syn::LitStr>()?;
                AttrKind::Homepage(string.value())
            }
            "no_homepage" => AttrKind::NoHomepage,
            "repository" => {
                input.parse::<Token![=]>().map_err(|_| {
                    syn::Error::new(key.span(), format!("`{key}` attribute requires a value"))
                })?;
                let string = input.parse::<syn::LitStr>()?;
                AttrKind::Repository(string.value())
            }
            "no_repository" => AttrKind::NoRepository,

            // Item/Variant-level attributes.
            "name" if target == Target::TopLevel || target == Target::Variant => {
                input.parse::<Token![=]>().map_err(|_| {
                    syn::Error::new(key.span(), format!("`{key}` attribute requires a value"))
                })?;
                let lit = input.parse::<syn::LitStr>()?;
                let name = lit.value();
                if name.starts_with('-') {
                    return Err(syn::Error::new(
                        lit.span(),
                        "(sub)command names must not start with `-`",
                    ));
                }
                AttrKind::Name(name)
            }
            "fallback" if target == Target::Variant => AttrKind::Fallback,
            "discover" if target == Target::Variant => {
                let path = if input.parse::<Option<Token![=]>>()?.is_some() {
                    Some(input.parse::<syn::LitStr>()?.parse()?)
                } else {
                    None
                };

                AttrKind::Discover(path)
            }

            // Field-level attributes.
            "name" if matches!(target, Target::Field) => {
                let mut short = None;
                let mut long = None;

                input.parse::<Token![=]>().map_err(|_| {
                    syn::Error::new(key.span(), format!("`{key}` attribute requires a value"))
                })?;
                let names = if input.lookahead1().peek(syn::LitStr) {
                    // `name = "-s"` or `name = "--long"`
                    let string = input.parse::<syn::LitStr>()?;
                    vec![string]
                } else {
                    // `name = [..]`
                    let content;
                    bracketed!(content in input);

                    let names =
                        content.parse_terminated(|s| s.parse::<syn::LitStr>(), Token![,])?;
                    names.into_iter().collect::<Vec<_>>()
                };

                for string in names {
                    let name = string
                        .value()
                        .parse::<ArgName>()
                        .map_err(|msg| syn::Error::new(string.span(), msg))?;
                    match name {
                        ArgName::Short(ch) => {
                            if short.is_some() {
                                return Err(syn::Error::new(
                                    string.span(),
                                    "only a single short name is allowed",
                                ));
                            }
                            short = Some(ch);
                        }
                        ArgName::Long(s) => {
                            if long.is_some() {
                                return Err(syn::Error::new(
                                    string.span(),
                                    "only a single long name is allowed",
                                ));
                            }
                            long = Some(s);
                        }
                    }
                }

                let names =
                    ArgNames::new(short, long).map_err(|msg| syn::Error::new(input.span(), msg))?;
                AttrKind::ArgName(names)
            }
            "default" => {
                let expr = if input.parse::<Option<Token![=]>>()?.is_some() {
                    Some(input.parse::<syn::LitStr>()?.value())
                } else {
                    None
                };

                AttrKind::Default(expr)
            }
            "inverse_of" => {
                input.parse::<Token![=]>().map_err(|_| {
                    syn::Error::new(key.span(), format!("`{key}` attribute requires a value"))
                })?;
                let inverse_of = input.parse::<syn::LitStr>()?.parse::<syn::Ident>()?;

                AttrKind::InverseOf(inverse_of)
            }
            "flag" => AttrKind::Flag,
            "subcommand" => AttrKind::Subcommand,
            "required" => AttrKind::Required,
            unk => {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown `larpa` attribute '{unk}'"),
                ));
            }
        };

        Ok(Attr { name: key, kind })
    }

    /// Parses all `#[larpa]` attributes on an item.
    pub fn parse_attrs(
        &mut self,
        attrs: &[syn::Attribute],
        target: Target,
    ) -> syn::Result<Vec<Attr>> {
        let mut out = Vec::new();
        for attr in attrs {
            if !attr.path().is_ident("larpa") {
                continue;
            }

            let list = attr.meta.require_list()?;
            let attrs = list.parse_args_with(|input: ParseStream<'_>| {
                // Hand-rolled `parse_terminated_with` function, since that requires an fn pointer, not a closure.
                let mut attrs = Vec::new();
                loop {
                    if input.is_empty() {
                        break;
                    }
                    let value = self.parse(input, target)?;
                    attrs.push(value);
                    if input.is_empty() {
                        break;
                    }
                    input.parse::<Token![,]>()?;
                }
                Ok(attrs)
            })?;
            out.extend(attrs);
        }

        for (i, attr) in out.iter().enumerate() {
            for attr2 in &out[i + 1..] {
                if attr.name.to_string() == attr2.name.to_string() {
                    let on = match target {
                        Target::TopLevel => "type",
                        Target::Variant => "variant",
                        Target::Field => "field",
                    };
                    return Err(syn::Error::new(
                        attr2.name.span(),
                        format!("duplicate `{}` attribute on this {on}", attr2.name),
                    ));
                }
            }
        }

        Ok(out)
    }

    pub fn crate_path(&self) -> Option<syn::Path> {
        self.krate.clone()
    }

    pub fn attr_refs(&self, private: &TokenStream2, mut f: impl FnMut(&TokenStream2)) {
        let top_level = quote!(#private::attrs::top_level::);
        let variant = quote!(#private::attrs::variant::);
        let field = quote!(#private::attrs::field::);
        for r in &self.refs {
            let m = match r.target {
                Target::TopLevel => &top_level,
                Target::Variant => &variant,
                Target::Field => &field,
            };
            let name = &r.name;
            if name.to_string() == "crate" {
                continue; // keywords don't work with this trick (even when replacing the "c" with a confusable)
            }
            f(&quote!(#m #name));
        }
    }
}

pub struct AttrRef {
    pub target: Target,
    pub name: syn::Ident,
}

pub fn doc_comment(attrs: &[syn::Attribute]) -> syn::Result<Option<String>> {
    let mut lines = Vec::new();

    for attr in attrs {
        let syn::Meta::NameValue(nv) = &attr.meta else {
            continue;
        };
        if nv.path.is_ident("doc") {
            match &nv.value {
                syn::Expr::Lit(lit) => {
                    if let syn::Lit::Str(s) = &lit.lit {
                        lines.push(s.value());
                        continue;
                    }
                }
                _ => {}
            }
            // FIXME: awkward code flow due to `if let` guards being unstable
            return Err(syn::Error::new(
                nv.value.span(),
                "invalid doc comment (expected string literal)",
            ));
        }
    }
    if lines.is_empty() {
        return Ok(None);
    }

    // Remove common indentation.
    let indent = lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim_start_matches(' ').len();
            let indent = line.len() - trimmed;

            if trimmed == 0 { None } else { Some(indent) }
        })
        .min()
        .unwrap_or(0);
    lines.iter_mut().for_each(|line| {
        if line.len() >= indent {
            line.drain(..indent);
        }
    });

    // Concatenate all lines together, replacing an empty line with two hard line breaks, and
    // prepending a hard break in front of lines that start with a non-alphabetic character.
    let mut doc = String::new();
    for i in 0..lines.len() {
        let line = &lines[i];
        let next = lines.get(i + 1);
        doc.push_str(line);
        match next {
            Some(next) => match next.chars().next() {
                Some(start) => {
                    if start.is_alphabetic() {
                        if !doc.ends_with('\n') {
                            doc.push(' ');
                        }
                    } else {
                        doc.push('\n');
                    }
                }
                None => {
                    // Next line is empty.
                    doc.push_str("\n\n");
                }
            },
            None => {}
        }
    }

    Ok(Some(doc))
}

#[cfg(test)]
mod tests {
    use expect_test::{Expect, expect};
    use proc_macro2::TokenStream as TokenStream2;
    use quote::quote;
    use syn::parse::Parser;

    use super::*;

    fn parse_attr(ts: TokenStream2) -> AttrKind {
        parse_attr_on(ts, Target::TopLevel)
    }

    fn parse_attr_on(ts: TokenStream2, target: Target) -> AttrKind {
        let attrs = syn::Attribute::parse_outer.parse2(ts).unwrap();
        assert_eq!(attrs.len(), 1);

        let mut attrs = Context::default().parse_attrs(&attrs, target).unwrap();
        attrs.pop().unwrap().kind
    }

    #[track_caller]
    fn assert_error(ts: TokenStream2, expect: Expect) {
        assert_error_on(ts, Target::TopLevel, expect);
    }

    #[track_caller]
    fn assert_error_on(ts: TokenStream2, target: Target, expect: Expect) {
        let attrs = syn::Attribute::parse_outer.parse2(ts).unwrap();

        match Context::default().parse_attrs(&attrs, target) {
            Ok(_) => panic!("parsing succeeded, but was expected to fail"),
            Err(e) => expect.assert_eq(&e.to_string()),
        }
    }

    fn check_docs(ts: TokenStream2, expect: Expect) {
        let attrs = syn::Attribute::parse_outer.parse2(ts).unwrap();
        let mut docs = doc_comment(&attrs).unwrap().unwrap();
        if docs.contains('\n') {
            docs.push('\n');
        }
        expect.assert_eq(&docs);
    }

    #[test]
    fn test_docs() {
        check_docs(
            quote! {
                ///
            },
            expect![[r#""#]],
        );
        check_docs(
            quote! {
                /// Test
            },
            expect![[r#"Test"#]],
        );
        check_docs(
            quote! {
                /// First.
                /// Second.
            },
            expect![[r#"First. Second."#]],
        );

        check_docs(
            quote! {
                /// First.
                ///
                /// Second.
            },
            expect![[r#"
                First.

                Second.
            "#]],
        );
        check_docs(
            quote! {
                /// List:
                /// 1. One
                /// 2. Two
                /// - Dash
                /// * Star
            },
            expect![[r#"
                List:
                1. One
                2. Two
                - Dash
                * Star
            "#]],
        );
    }

    #[test]
    fn unknown() {
        assert_error(
            quote!(#[larpa(unknown_attribute)]),
            expect!["unknown `larpa` attribute 'unknown_attribute'"],
        );
        assert_error(
            quote!(#[larpa(r#name)]),
            expect!["unknown `larpa` attribute 'r#name'"],
        );
    }

    #[test]
    fn parse_crate() {
        assert_eq!(
            parse_attr(quote!(#[larpa(crate = "crate")])),
            AttrKind::Crate(syn::parse_str("crate").unwrap())
        );
        assert_eq!(
            parse_attr(quote!(#[larpa(crate = "::renamed")])),
            AttrKind::Crate(syn::parse_str("::renamed").unwrap())
        );
        assert_error(
            quote!(#[larpa(crate)]),
            expect!["`crate` attribute requires a value"],
        );
        assert_error(
            quote!(#[larpa(crate = crate)]),
            expect!["expected string literal"],
        );
    }

    #[test]
    fn duplicate() {
        assert_error_on(
            quote!(#[larpa(crate = "crate", crate = "::renamed")]),
            Target::TopLevel,
            expect!["duplicate `crate` attribute on this type"],
        );
        assert_error_on(
            quote!(#[larpa(crate = "crate")] #[larpa(crate = "::renamed")]),
            Target::TopLevel,
            expect!["duplicate `crate` attribute on this type"],
        );
        assert_error_on(
            quote!(#[larpa(name = "sub1", name = "sub2")]),
            Target::Variant,
            expect!["duplicate `name` attribute on this variant"],
        );
        assert_error_on(
            quote!(#[larpa(name = "--long", name = "-s")]),
            Target::Field,
            expect!["duplicate `name` attribute on this field"],
        );
    }

    #[test]
    fn parse_name() {
        // (Sub)command name:
        assert_eq!(
            parse_attr(quote!(#[larpa(name = "bla")])),
            AttrKind::Name("bla".into()),
        );
        assert_error(
            quote!(#[larpa(name)]),
            expect!["`name` attribute requires a value"],
        );
        assert_error(
            quote!(#[larpa(name = "-subcmd")]),
            expect!["(sub)command names must not start with `-`"],
        );

        // Argument name:
        assert_eq!(
            parse_attr_on(quote!(#[larpa(name = "-n")]), Target::Field),
            AttrKind::ArgName(ArgNames::new('n', None).unwrap()),
        );
        assert_eq!(
            parse_attr_on(quote!(#[larpa(name = "--long")]), Target::Field),
            AttrKind::ArgName(ArgNames::new(None, "long".to_string()).unwrap()),
        );
        assert_eq!(
            parse_attr_on(quote!(#[larpa(name = ["-n"])]), Target::Field),
            AttrKind::ArgName(ArgNames::new('n', None).unwrap()),
        );
        assert_eq!(
            parse_attr_on(quote!(#[larpa(name = ["--long", "-s"])]), Target::Field),
            AttrKind::ArgName(ArgNames::new('s', "long".to_string()).unwrap()),
        );
        assert_error_on(
            quote!(#[larpa(name = "nodash")]),
            Target::Field,
            expect![["invalid argument name (format: `-s` or `--long`)"]],
        );
        assert_error_on(
            quote!(#[larpa(name = "")]),
            Target::Field,
            expect![["invalid argument name (format: `-s` or `--long`)"]],
        );
        assert_error_on(
            quote!(#[larpa(name = "-toolong")]),
            Target::Field,
            expect![["short argument name must consist of a single character"]],
        );
    }

    #[test]
    fn parse_generate() {
        assert_eq!(
            parse_attr(quote!(#[larpa(no_generate_tests)])),
            AttrKind::NoGenerateTests
        );
    }

    #[test]
    fn superfluous_value() {
        assert_error(quote!(#[larpa(flag = "???")]), expect![[r#"expected `,`"#]]);
    }
}
