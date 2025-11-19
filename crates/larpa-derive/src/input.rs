use std::collections::BTreeMap;

use proc_macro2::Span;
use quote::quote;
use syn::spanned::Spanned;

use crate::{
    Metadata,
    attr::{AttrKind, Context, Target, doc_comment},
    default_crate_path,
};

/// Parsed input for `#[derive(Command)]`.
pub struct Input {
    pub ident: syn::Ident,

    /// Path to the `larpa` runtime crate.
    pub krate: syn::Path,

    pub emit_tests: bool,

    /// Variants of the `enum`, or a single variant if on a `struct`.
    pub variants: Vec<Variant>,

    /// If this is an `enum` with a `#[larpa(fallback)]` variant, stores that variant's name.
    pub fallback_variant: Option<FallbackVariant>,

    pub is_enum: bool,

    pub canonical_name: Option<String>,
    pub description: Option<String>,
    pub version: Option<syn::Expr>,
    pub version_fmt: Option<syn::Path>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
}

pub struct FallbackVariant {
    pub name: syn::Ident,
    pub discovery: Option<DiscoveryFn>,
}

impl Input {
    pub fn parse(cx: &mut Context, input: &syn::DeriveInput) -> syn::Result<Self> {
        let mut krate = None;
        let mut name = None;
        let mut version = None;
        let mut version_fmt = None;
        let mut license = None;
        let mut homepage = None;
        let mut repository = None;
        let mut test = true;
        let mut auto_help = true;

        for attr in cx.parse_attrs(&input.attrs, Target::TopLevel)? {
            match attr.kind {
                AttrKind::Crate(path) => {
                    krate = Some(path);
                }
                AttrKind::Name(n) => {
                    name = Some(n);
                }
                AttrKind::Version(v) => {
                    version = Some(v);
                }
                AttrKind::VersionFormatter(fmt) => {
                    version_fmt = Some(fmt);
                }
                AttrKind::NoGenerateHelp => {
                    auto_help = false;
                }
                AttrKind::NoGenerateTests => {
                    test = false;
                }
                AttrKind::License(lic) => {
                    if license.is_some() {
                        return Err(syn::Error::new(
                            attr.name.span(),
                            "`license` cannot be combined with `no_license`",
                        ));
                    }
                    license = Some(Some(lic));
                }
                AttrKind::NoLicense => {
                    if license.is_some() {
                        return Err(syn::Error::new(
                            attr.name.span(),
                            "`license` cannot be combined with `no_license`",
                        ));
                    }
                    license = Some(None);
                }
                AttrKind::Homepage(hp) => {
                    if homepage.is_some() {
                        return Err(syn::Error::new(
                            attr.name.span(),
                            "`homepage` cannot be combined with `no_homepage`",
                        ));
                    }
                    homepage = Some(Some(hp));
                }
                AttrKind::NoHomepage => {
                    if homepage.is_some() {
                        return Err(syn::Error::new(
                            attr.name.span(),
                            "`homepage` cannot be combined with `no_homepage`",
                        ));
                    }
                    homepage = Some(None);
                }
                AttrKind::Repository(repo) => {
                    if repository.is_some() {
                        return Err(syn::Error::new(
                            attr.name.span(),
                            "`repository` cannot be combined with `no_repository`",
                        ));
                    }
                    repository = Some(Some(repo));
                }
                AttrKind::NoRepository => {
                    if repository.is_some() {
                        return Err(syn::Error::new(
                            attr.name.span(),
                            "`repository` cannot be combined with `no_repository`",
                        ));
                    }
                    repository = Some(None);
                }
                _ => {
                    return Err(syn::Error::new(
                        attr.name.span(),
                        "this attribute is not valid on a top-level declaration",
                    ));
                }
            }
        }

        let krate = krate.unwrap_or_else(default_crate_path);

        let mut fallback_variant = None;
        let mut variants = match &input.data {
            syn::Data::Struct(data_struct) => {
                vec![Variant::for_struct(cx, data_struct, input.ident.clone())?]
            }
            syn::Data::Union(data_union) => {
                return Err(syn::Error::new(
                    data_union.union_token.span,
                    "`larpa` does not support unions",
                ));
            }
            syn::Data::Enum(data_enum) => {
                let mut out = Vec::new();
                for (i, variant) in data_enum.variants.iter().enumerate() {
                    let is_last = i == data_enum.variants.len() - 1;

                    let v = Variant::for_enum(cx, variant)?;
                    match &v.kind {
                        VariantKind::Fallback { discovery } => {
                            if !is_last {
                                return Err(syn::Error::new(
                                    v.ident.span(),
                                    "`#[larpa(fallback)]` is only allowed on the last variant of an enum",
                                ));
                            }

                            fallback_variant = Some(FallbackVariant {
                                name: v.ident.clone(),
                                discovery: discovery.clone(),
                            });
                        }
                        VariantKind::Command | VariantKind::Wrapped(_) => out.push(v),
                    }
                }

                out
            }
        };

        for variant in &mut variants {
            if auto_help
                && variant
                    .args
                    .all_args
                    .iter()
                    .all(|arg| arg.long() != Some("help"))
            {
                variant.args.synth_args.push(Arg {
                    description: Some("Print help information.".into()),
                    ..Arg::synth(
                        "help",
                        syn::parse2(quote! {
                            #krate::types::PrintHelp
                        })
                        .unwrap(),
                    )
                });
            }
        }

        let meta = Metadata::get();
        let license = license.unwrap_or(meta.pkg_license);
        let homepage = homepage.unwrap_or(meta.pkg_homepage);
        let repository = repository.unwrap_or(meta.pkg_repository);
        Ok(Self {
            ident: input.ident.clone(),
            krate,
            emit_tests: test,
            variants,
            fallback_variant,
            is_enum: matches!(input.data, syn::Data::Enum(_)),
            canonical_name: name,
            description: doc_comment(&input.attrs)?,
            version,
            version_fmt,
            license,
            homepage,
            repository,
        })
    }
}

pub struct Variant {
    pub ident: syn::Ident,
    pub subcommand_name: String,
    pub description: Option<String>,
    pub args: Args,
    pub kind: VariantKind,
}

pub enum VariantKind {
    /// Normal command or subcommand.
    Command,

    /// Tuple variant that wraps another `Command` (`inner`).
    Wrapped(syn::Type),

    /// Tuple variant marked with `#[larpa(fallback)]`.
    Fallback { discovery: Option<DiscoveryFn> },
}

#[derive(Clone)]
pub enum DiscoveryFn {
    /// The default discovery function was requested.
    Default,
    /// A custom subcommand discovery function was provided.
    Custom(syn::Path),
}

impl Variant {
    fn for_struct(
        cx: &mut Context,
        strukt: &syn::DataStruct,
        ident: syn::Ident,
    ) -> syn::Result<Self> {
        Ok(Self {
            subcommand_name: variant_name_to_subcommand_name(&ident.to_string()),
            ident,
            description: None,
            args: Args::parse(cx, &strukt.fields)?,
            kind: VariantKind::Command,
        })
    }

    fn for_enum(cx: &mut Context, variant: &syn::Variant) -> syn::Result<Self> {
        let mut is_fallback = false;
        let mut subcommand_name = None;
        let mut discover_span = None;
        let mut discover_path = None;
        for attr in cx.parse_attrs(&variant.attrs, Target::Variant)? {
            match attr.kind {
                AttrKind::Name(name) => subcommand_name = Some(name),
                AttrKind::Fallback => is_fallback = true,
                AttrKind::Discover(path) => {
                    discover_span = Some(attr.name.span());
                    discover_path = Some(
                        path.map(DiscoveryFn::Custom)
                            .unwrap_or(DiscoveryFn::Default),
                    );
                }
                _ => {
                    return Err(syn::Error::new(
                        attr.name.span(),
                        "this attribute is not allowed on enum variants",
                    ));
                }
            }
        }

        if let Some(span) = discover_span
            && !is_fallback
        {
            return Err(syn::Error::new(
                span,
                "`#[larpa(discover)]` must be used with `#[larpa(fallback)]`",
            ));
        }

        let (kind, fields) = if is_fallback {
            let invalid = match &variant.fields {
                syn::Fields::Unnamed(flds) => flds.unnamed.len() != 1,
                _ => true,
            };

            if invalid {
                return Err(syn::Error::new(
                    variant.ident.span(),
                    "`#[larpa(fallback)]` must be on a tuple variant with a single `Vec<OsString>` field",
                ));
            }

            let kind = VariantKind::Fallback {
                discovery: discover_path,
            };
            (kind, Args::default())
        } else {
            match &variant.fields {
                syn::Fields::Unnamed(flds) => {
                    if flds.unnamed.len() != 1 {
                        let msg =
                            "tuple variants must wrap exactly one type that implements `Command`";
                        return Err(syn::Error::new(flds.span(), msg));
                    }

                    let kind = VariantKind::Wrapped(flds.unnamed.first().unwrap().ty.clone());
                    (kind, Args::default())
                }
                syn::Fields::Named(_) => (VariantKind::Command, Args::parse(cx, &variant.fields)?),
                syn::Fields::Unit => (VariantKind::Command, Args::default()),
            }
        };

        Ok(Self {
            ident: variant.ident.clone(),
            description: doc_comment(&variant.attrs)?,
            args: fields,
            kind,
            subcommand_name: subcommand_name
                .unwrap_or_else(|| variant_name_to_subcommand_name(&variant.ident.to_string())),
        })
    }
}

fn variant_name_to_subcommand_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_uppercase() {
            // Start of a new word.
            if !out.is_empty() {
                out.push('-');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[derive(Default)]
pub struct Args {
    pub all_args: Vec<Arg>,
    /// Synthetic arguments that do not correspond to a field in the struct/variant.
    pub synth_args: Vec<Arg>,
    pub subcommand: Option<SubcommandField>,
}

pub struct SubcommandField {
    pub ident: syn::Ident,
    pub is_optional: bool,
    /// Unwrapped type of the inner subcommand. Implements `Command` and `Subcommands`.
    pub cmd_type: syn::Type,
}

enum Field {
    Arg(Arg),
    Subcommand(SubcommandField),
}

#[derive(Clone)]
pub struct Arg {
    pub description: Option<String>,
    pub field_name: syn::Ident,
    pub value_name: String,
    pub inner_type: syn::Type,
    pub wrapper_type: Option<WrapperType>,
    pub default: Option<DefaultValue>,
    pub inverse_of: Option<syn::Ident>,
    pub kind: ArgKind,
    pub required: bool,
    pub repeating: bool,
}

impl Arg {
    fn synth(long: &str, ty: syn::Type) -> Self {
        Self {
            description: None,
            field_name: syn::Ident::new(&format!("__{long}"), Span::call_site()),
            value_name: String::new(),
            inner_type: ty,
            wrapper_type: None,
            default: None,
            inverse_of: None,
            kind: ArgKind::Named {
                short: None,
                long: Some(long.into()),
                is_flag: true,
            },
            required: false,
            repeating: false,
        }
    }

    pub fn is_flag(&self) -> bool {
        match &self.kind {
            ArgKind::Positional => false,
            ArgKind::Named { is_flag, .. } => *is_flag,
        }
    }

    pub fn is_positional(&self) -> bool {
        match &self.kind {
            ArgKind::Positional => true,
            ArgKind::Named { .. } => false,
        }
    }

    pub fn is_optional(&self) -> bool {
        !self.required
    }

    pub fn short(&self) -> Option<char> {
        match &self.kind {
            ArgKind::Positional => None,
            ArgKind::Named { short, .. } => *short,
        }
    }

    pub fn long(&self) -> Option<&str> {
        match &self.kind {
            ArgKind::Positional => None,
            ArgKind::Named { long, .. } => long.as_deref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgKind {
    Positional,
    Named {
        // Either short or long will be `Some` if parsing succeeds. Neither contains the preceding `-` or `--`.
        short: Option<char>,
        long: Option<String>,
        is_flag: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapperType {
    Option,
    Vec,
}

#[derive(Clone, PartialEq, Eq)]
pub enum DefaultValue {
    /// Use `Default::default()`.
    Default,
    /// Parse default value from this string.
    String(String),
}

impl Args {
    fn parse(cx: &mut Context, fields: &syn::Fields) -> syn::Result<Self> {
        let named = match fields {
            syn::Fields::Unnamed(_) => {
                return Err(syn::Error::new(
                    fields.span(),
                    "tuple structs/variants are not supported by `larpa`; use a struct/variant with named fields instead",
                ));
            }
            syn::Fields::Unit => return Ok(Self::default()),
            syn::Fields::Named(named) => named,
        };

        let mut positionals = Vec::new();
        let mut subcommand = None;
        let mut short_args = BTreeMap::new();
        let mut long_args = BTreeMap::new();
        let mut last_positional = None;
        let mut all_args = Vec::new();

        for field in &named.named {
            let parsed_field = Field::parse(cx, field)?;
            let arg = match parsed_field {
                Field::Subcommand(subcmd) => {
                    if subcommand.is_some() {
                        return Err(syn::Error::new(
                            field.span(),
                            "duplicate `subcommand` field (only one is permitted)",
                        ));
                    }

                    subcommand = Some(subcmd);
                    continue;
                }
                Field::Arg(arg) => arg,
            };

            all_args.push(arg.clone());
            match &arg.kind {
                ArgKind::Positional => {
                    let Some(prev) = &last_positional else {
                        last_positional = Some(arg.clone());
                        positionals.push(arg);
                        continue;
                    };

                    // If there are multiple positional arguments, there are certain constraints we
                    // need to uphold to allow parsing.
                    // - Optional or repeated positionals must not be followed by required positionals.
                    // - Repeated positionals must not be followed by any other positionals.

                    if prev.repeating {
                        let mut error = syn::Error::new(
                            arg.field_name.span(),
                            "repeating positional arguments must not be followed by any other positional arguments",
                        );
                        error.combine(syn::Error::new(
                            prev.field_name.span(),
                            "previous repeating positional argument here",
                        ));
                        return Err(error);
                    }

                    if (prev.is_optional() || prev.repeating) && arg.required {
                        let mut error = syn::Error::new(
                            arg.field_name.span(),
                            "required positional arguments cannot follow optional or repeating positional arguments",
                        );
                        error.combine(syn::Error::new(
                            prev.field_name.span(),
                            "previous positional argument here",
                        ));
                        return Err(error);
                    }

                    last_positional = Some(arg.clone());
                    positionals.push(arg);
                }
                ArgKind::Named { short, long, .. } => {
                    if let Some(short) = *short {
                        if let Some(prev) = short_args.insert(short, arg.clone()) {
                            let mut error = syn::Error::new(
                                arg.field_name.span(),
                                format!(
                                    "field `{}` uses short flag name '{short}', which is already in use",
                                    arg.field_name
                                ),
                            );
                            error.combine(syn::Error::new(
                                prev.field_name.span(),
                                format!("previous use of short flag '{short}' here"),
                            ));
                            return Err(error);
                        }
                    }

                    if let Some(long) = long.clone() {
                        // this is what clap falls back to
                        let field_name = arg.field_name.clone();
                        if let Some(prev) = long_args.insert(long.to_string(), arg) {
                            let mut error = syn::Error::new(
                                field_name.span(),
                                format!(
                                    "field `{}` uses long flag name '{long}', which is already in use",
                                    field_name
                                ),
                            );
                            error.combine(syn::Error::new(
                                prev.field_name.span(),
                                format!("previous use of long flag '{long}' here"),
                            ));
                            return Err(error);
                        }
                    }
                }
            }
        }

        if let Some(subcmd) = &subcommand {
            if let Some(arg) = last_positional
                && (arg.repeating || arg.is_optional())
            {
                let mut error = syn::Error::new(
                    arg.field_name.span(),
                    "repeating or optional positional arguments cannot be combined with `subcommand`",
                );
                error.combine(syn::Error::new(
                    subcmd.ident.span(),
                    "repeating or optional positional arguments cannot be combined with `subcommand`",
                ));
                return Err(error);
            }
        }

        // Validate usage of `inverse_of`.
        for arg in &all_args {
            let Some(inv_of) = &arg.inverse_of else {
                continue;
            };

            let Some(inverse_of) = all_args.iter().find(|arg| arg.field_name == *inv_of) else {
                return Err(syn::Error::new(
                    inv_of.span(),
                    format!("field `{inv_of}` does not exist"),
                ));
            };

            if !inverse_of.is_flag() {
                return Err(syn::Error::new(
                    inv_of.span(),
                    "`inverse_of` must refer to a field with `#[larpa(flag)]`",
                ));
            }
        }

        Ok(Self {
            all_args,
            synth_args: Vec::new(),
            subcommand,
        })
    }
}

impl Field {
    fn parse(cx: &mut Context, field: &syn::Field) -> syn::Result<Self> {
        let Some(field_name) = &field.ident else {
            return Err(syn::Error::new(
                field.ty.span(),
                "tuple structs and variants are not supported by the `larpa` crate",
            ));
        };

        if field_name.to_string().starts_with("__") {
            return Err(syn::Error::new(
                field_name.span(),
                "fields must not start with `__` when using `#[derive(Command)]`",
            ));
        }

        let mut subcommand = None;
        let mut names = None;
        let mut default = None;
        let mut flag = None;
        let mut inverse_of = None;
        let mut required = None;

        let attrs = cx.parse_attrs(&field.attrs, Target::Field)?;
        let attr_count = attrs.len();
        for attr in attrs {
            match attr.kind {
                AttrKind::ArgName(n) => {
                    names = Some(n);
                }
                AttrKind::Default(dfl) => {
                    default = Some(match dfl {
                        Some(str) => DefaultValue::String(str),
                        None => DefaultValue::Default,
                    });
                }
                AttrKind::Flag => {
                    flag = Some(attr.name.span());
                }
                AttrKind::InverseOf(field) => {
                    inverse_of = Some(field);
                }
                AttrKind::Subcommand => {
                    subcommand = Some(attr.name.span());
                }
                AttrKind::Required => required = Some(attr.name.span()),

                _ => {
                    return Err(syn::Error::new(
                        attr.name.span(),
                        format!("`{}` attribute is not allowed on fields", attr.name),
                    ));
                }
            }
        }

        // `subcommand` is incompatible with everything else.
        if let Some(subcommand) = subcommand {
            if attr_count != 1 {
                return Err(syn::Error::new(
                    subcommand,
                    "`subcommand` cannot be combined with other attributes",
                ));
            }

            let (cmd_type, optional);
            match unwrap_type(&field.ty) {
                Some((ty, WrapperType::Option)) => {
                    optional = true;
                    cmd_type = ty;
                }
                _ => {
                    optional = false;
                    cmd_type = field.ty.clone();
                }
            }

            return Ok(Field::Subcommand(SubcommandField {
                ident: field.ident.clone().unwrap(),
                is_optional: optional,
                cmd_type,
            }));
        }

        match (flag, &default) {
            (Some(flag_span), Some(_)) => {
                return Err(syn::Error::new(
                    flag_span,
                    "`flag` and `default` cannot be used together",
                ));
            }
            _ => {}
        }

        match (flag, &names) {
            (Some(flag_span), None) => {
                return Err(syn::Error::new(
                    flag_span,
                    "`flag` is not allowed on positional arguments; use `name` to make it a named flag",
                ));
            }
            _ => {}
        }

        match (&default, &required) {
            (Some(_), Some(req)) => {
                return Err(syn::Error::new(
                    req.span(),
                    "`required` and `default` cannot be used together",
                ));
            }
            _ => {}
        }

        match (flag, &inverse_of) {
            (None, Some(ident)) => {
                return Err(syn::Error::new(
                    ident.span(),
                    "`inverse_of` must be used with `flag`",
                ));
            }
            _ => {}
        }
        if let Some(ident) = &inverse_of {
            if *ident == field.ident.clone().unwrap() {
                return Err(syn::Error::new(
                    ident.span(),
                    "fields with `inverse_of` cannot refer to themselves",
                ));
            }

            match &field.ty {
                syn::Type::Tuple(tup) if tup.elems.is_empty() => {}
                _ => {
                    return Err(syn::Error::new(
                        field.ty.span(),
                        "fields with `inverse_of` must have type `()`",
                    ));
                }
            }
        }

        let arg_kind = match names {
            None => ArgKind::Positional,
            Some(names) => ArgKind::Named {
                short: names.short(),
                long: names.long().map(Into::into),
                is_flag: flag.is_some(),
            },
        };

        let (inner_type, wrapper, repeating) = if flag.is_some() {
            (field.ty.clone(), None, true)
        } else {
            match unwrap_type(&field.ty) {
                Some((inner, wrapper @ WrapperType::Option)) => (inner, Some(wrapper), false),
                Some((inner, wrapper @ WrapperType::Vec)) => (inner, Some(wrapper), true),
                None => (field.ty.clone(), None, false),
            }
        };

        if default.is_some() && wrapper.is_some() {
            return Err(syn::Error::new(
                field_name.span(),
                "`default` is not allowed when the field type is wrapped in `Option<_>` or `Vec<_>`; choose one or the other",
            ));
        }

        if required.is_some() && wrapper != Some(WrapperType::Vec) {
            return Err(syn::Error::new(
                field_name.span(),
                "`required` is only allowed on fields wrapped in `Vec<_>`",
            ));
        }

        Ok(Field::Arg(Arg {
            description: doc_comment(&field.attrs)?,
            field_name: field_name.clone(),
            value_name: field_name_to_value_name(field_name),
            inner_type,
            wrapper_type: wrapper,
            required: required.is_some()
                || (!flag.is_some() && default.is_none() && wrapper.is_none()),
            default,
            inverse_of,
            kind: arg_kind,
            repeating,
        }))
    }
}

fn field_name_to_value_name(name: &syn::Ident) -> String {
    // "value names" are the names of positional arguments, as well as the placeholder value
    // associated with a named argument that takes a value.
    // `--long <VALUE>` <- "VALUE" is the value name
    let name = &name.to_string();
    let mut trimmed = name.trim_start_matches('_');
    if trimmed.is_empty() {
        // Orig. name consisted only of `_`. Don't trim. Not sure what bit you're doing, hope it works out?
        trimmed = name;
    }
    trimmed.to_uppercase()
}

fn unwrap_type(ty: &syn::Type) -> Option<(syn::Type, WrapperType)> {
    match ty {
        syn::Type::Path(tpath) => {
            if tpath.qself.is_some() {
                return None;
            }
            if tpath.path.segments.len() != 1 {
                return None;
            }

            let segment = &tpath.path.segments[0];
            let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                return None;
            };
            if args.args.len() != 1 {
                return None;
            }
            let arg = args.args.first().unwrap();
            let syn::GenericArgument::Type(ty) = arg else {
                return None;
            };

            if segment.ident == "Vec" {
                Some((ty.clone(), WrapperType::Vec))
            } else if segment.ident == "Option" {
                Some((ty.clone(), WrapperType::Option))
            } else {
                None
            }
        }
        _ => None,
    }
}

// TODO: test raw identifiers

#[cfg(test)]
mod tests {
    use expect_test::{Expect, expect};
    use quote::quote;

    use super::*;

    fn parse_fields(tokens: proc_macro2::TokenStream) -> syn::Result<Args> {
        let strukt = quote! {
            struct S {
                #tokens
            }
        };
        let strukt: syn::DeriveInput = syn::parse2(strukt).unwrap();
        let field = match strukt.data {
            syn::Data::Struct(strukt) => strukt.fields,
            _ => unreachable!(),
        };
        Args::parse(&mut Context::default(), &field)
    }

    #[track_caller]
    fn assert_error(tokens: proc_macro2::TokenStream, expect: Expect) {
        let error = parse_fields(tokens)
            .err()
            .expect("expected error, got successful parse");
        expect.assert_eq(&error.to_string());
    }

    fn assert_ok(tokens: proc_macro2::TokenStream) -> Args {
        parse_fields(tokens).unwrap()
    }

    fn assert_error_top_level(tokens: proc_macro2::TokenStream, expect: Expect) {
        let strukt: syn::DeriveInput = syn::parse2(tokens).unwrap();
        let mut cx = Context::default();
        let error = Input::parse(&mut cx, &strukt)
            .err()
            .expect("expected error, got successful parse");
        expect.assert_eq(&error.to_string());
    }

    // FIXME: messy. redundant. consolidate.

    #[test]
    fn rejects_duplicate_attrs() {
        assert_error(
            quote! {
                #[larpa(name = "-f", default, name = "--asdf")]
                field: u8,
            },
            expect!["duplicate `name` attribute on this field"],
        );
        assert_error(
            quote! {
                #[larpa(name = "--field")]
                #[larpa(name = "--field")]
                field: u8,
            },
            expect!["duplicate `name` attribute on this field"],
        );
    }

    #[test]
    fn rejects_conflicting_metadata() {
        assert_error_top_level(
            quote! {
                #[larpa(no_license, license = "bla")]
                struct S;
            },
            expect!["`license` cannot be combined with `no_license`"],
        );
        assert_error_top_level(
            quote! {
                #[larpa(no_homepage, homepage = "bla")]
                struct S;
            },
            expect!["`homepage` cannot be combined with `no_homepage`"],
        );
        assert_error_top_level(
            quote! {
                #[larpa(repository = "bla", no_repository, )]
                struct S;
            },
            expect!["`repository` cannot be combined with `no_repository`"],
        );
    }

    #[test]
    fn rejects_default_on_wrapped_types() {
        assert_error(
            quote! {
                #[larpa(name = "-f", default)]
                field: Option<u8>,
            },
            expect![
                "`default` is not allowed when the field type is wrapped in `Option<_>` or `Vec<_>`; choose one or the other"
            ],
        );
        assert_error(
            quote! {
                #[larpa(name = "-f", default)]
                field: Vec<u8>,
            },
            expect![
                "`default` is not allowed when the field type is wrapped in `Option<_>` or `Vec<_>`; choose one or the other"
            ],
        );
    }

    #[test]
    fn rejects_duplicate_flags() {
        assert_error(
            quote! {
                #[larpa(name = "-a")]
                asdf: u8,
                #[larpa(name = "-a")]
                asdg: u8,
            },
            expect!["field `asdg` uses short flag name 'a', which is already in use"],
        );

        assert_error(
            quote! {
                #[larpa(name = "--asdf")]
                asdf: u8,
                #[larpa(name = "--asdf")]
                asdg: u8,
            },
            expect!["field `asdg` uses long flag name 'asdf', which is already in use"],
        );
    }

    #[test]
    fn rejects_invalid_positionals() {
        // If there are both optional/repeating and required positional arguments, all optional/repeating ones must come last.
        assert_error(
            quote! {
                opt: Option<u8>,
                req: String,
            },
            expect![
                "required positional arguments cannot follow optional or repeating positional arguments"
            ],
        );

        assert_error(
            quote! {
                #[larpa(default)]
                opt: u8,
                req: u8,
            },
            expect![
                "required positional arguments cannot follow optional or repeating positional arguments"
            ],
        );

        // Once there's a repeating positional, no other positionals can follow.
        assert_error(
            quote! {
                repeat: Vec<u8>,
                req: String,
            },
            expect![
                "repeating positional arguments must not be followed by any other positional arguments"
            ],
        );

        assert_error(
            quote! {
                repeat: Vec<u8>,
                optional: Option<String>,
            },
            expect![
                "repeating positional arguments must not be followed by any other positional arguments"
            ],
        );

        assert_error(
            quote! {
                repeat: Vec<u8>,
                repeat_2: Vec<String>,
            },
            expect![
                "repeating positional arguments must not be followed by any other positional arguments"
            ],
        );

        // Two subsequent optional positionals are ok.
        assert_ok(quote! {
            req: u8,
            opt1: Option<u8>,
            opt2: Option<String>,
        });
        assert_ok(quote! {
            req: u8,
            opt1: Option<u8>,
            rep: Vec<String>,
        });
    }

    #[test]
    fn rejects_invalid_subcommand() {
        assert_error(
            quote! {
                #[larpa(subcommand)]
                sub1: u8,
                #[larpa(subcommand)]
                sub2: u8,
            },
            expect!["duplicate `subcommand` field (only one is permitted)"],
        );

        assert_error(
            quote! {
                #[larpa(subcommand, subcommand)]
                sub1: u8,
            },
            expect!["duplicate `subcommand` attribute on this field"],
        );

        assert_error(
            quote! {
                #[larpa(subcommand, name = "-s")]
                sub1: u8,
            },
            expect!["`subcommand` cannot be combined with other attributes"],
        );

        assert_error(
            quote! {
                #[larpa(subcommand)]
                sub1: u8,

                positionals: Vec<u8>,
            },
            expect![
                "repeating or optional positional arguments cannot be combined with `subcommand`"
            ],
        );
    }

    #[test]
    fn rejects_invalid_flags() {
        assert_error(
            quote! {
                #[larpa(name = "-f", flag, default)]
                flg: u8,
            },
            expect!["`flag` and `default` cannot be used together"],
        );
    }

    #[test]
    fn rejects_invalid_inverse_of() {
        assert_error(
            quote! {
                #[larpa(flag, name = "-f", inverse_of = "flg")]
                flg: u8,
            },
            expect!["fields with `inverse_of` cannot refer to themselves"],
        );

        assert_error(
            quote! {
                #[larpa(flag, name = "-f")]
                flg: u8,

                #[larpa(flag, name = "-i", inverse_of = "flg")]
                inv: not_unit,
            },
            expect!["fields with `inverse_of` must have type `()`"],
        );
        assert_error(
            quote! {
                #[larpa(flag, name = "-f")]
                flg: u8,

                #[larpa(flag, name = "-i", inverse_of = "flg")]
                inv: (u8,),
            },
            expect!["fields with `inverse_of` must have type `()`"],
        );

        assert_error(
            quote! {
                #[larpa(flag, name = "-f", inverse_of = "doesnotexist")]
                inv: (),
            },
            expect!["field `doesnotexist` does not exist"],
        );
        assert_error(
            quote! {
                #[larpa(name = "-f")]
                flg: u8,

                #[larpa(flag, name = "-i", inverse_of = "flg")]
                inv: (),
            },
            expect!["`inverse_of` must refer to a field with `#[larpa(flag)]`"],
        );
    }

    #[test]
    fn test_unwrap_type() {
        assert!(unwrap_type(&syn::parse_str("std::vec::Vec<u8>").unwrap()).is_none());
        assert!(unwrap_type(&syn::parse_str("core::vec::Vec<u8>").unwrap()).is_none());
        assert!(unwrap_type(&syn::parse_str("Vec<'a, u8>").unwrap()).is_none());
        assert!(unwrap_type(&syn::parse_str("Vec<1>").unwrap()).is_none());
        assert_eq!(
            unwrap_type(&syn::parse_str("Vec<u8>").unwrap()).unwrap().1,
            WrapperType::Vec,
        );
        assert_eq!(
            unwrap_type(&syn::parse_str("Option<u8>").unwrap())
                .unwrap()
                .1,
            WrapperType::Option,
        );
    }

    #[test]
    fn subcommand_name() {
        assert_eq!(variant_name_to_subcommand_name("Help"), "help");
        assert_eq!(variant_name_to_subcommand_name("ListUsers"), "list-users");
    }
}
