use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote};
use syn::spanned::Spanned;

use crate::{
    Context, Input, Metadata,
    input::{Arg, DefaultValue, DiscoveryFn, SubcommandField, Variant, VariantKind, WrapperType},
    respan::Respan,
};

pub struct Codegen<'a> {
    cx: &'a Context,
    input: &'a Input,
    meta: &'a Metadata,
    /// Name of the input type, converted to something snake_case compatible.
    snake_case: syn::Ident,
    private: TokenStream2,
    none: TokenStream2,
    some: TokenStream2,
    unit_test: TokenStream2,
}

impl<'a> Codegen<'a> {
    pub fn new(cx: &'a Context, input: &'a Input, meta: &'a Metadata) -> Self {
        let krate = &input.krate;
        let snake_case = syn::Ident::new(
            &input.ident.to_string().to_lowercase().trim_matches('_'),
            input.ident.span(),
        );
        Self {
            cx,
            input,
            meta,
            snake_case,
            private: quote!(#krate::private),
            none: quote!(::core::option::Option::None),
            some: quote!(::core::option::Option::Some),
            unit_test: TokenStream2::new(),
        }
    }

    fn none(&self) -> &TokenStream2 {
        &self.none
    }

    fn some<V: ToTokens>(&self, inner: V) -> TokenStream2 {
        let some = &self.some;
        quote!(#some(#inner))
    }

    pub fn derive(&mut self) -> syn::Result<TokenStream2> {
        let desc = self.build_command_desc()?;

        let private = &self.private;
        let ident = &self.input.ident;

        // The actual parser (one per variant, in case of `enum`s).
        // Consumes args from the raw argument parser.
        let variant_parsers = self.input.variants.iter().map(|v| {
            // List of variant fields that we want to "fake use" so the `dead_code` lint doesn't trigger on them.
            let mut alive_fields = Vec::new();

            match &v.kind {
                VariantKind::Wrapped(ty) => {
                    // `Variant(Subcommand)` forwards to the inner type's implementation.
                    let variant = &v.ident;
                    return quote! {
                        #private::Ok(#ident::#variant(<#ty as #private::CommandInternal>::parse(__cx, __p)?))
                    };
                },
                VariantKind::Fallback { .. } => unreachable!(),
                VariantKind::Command => {},
            }

            let field_decls = v.args.all_args.iter().chain(v.args.synth_args.iter()).map(|arg| {
                let inner_ty = &arg.inner_type;
                let ty = match arg.wrapper_type {
                    Some(WrapperType::Vec) => quote!( Vec<#inner_ty> ),
                    _ => {
                        if arg.is_flag() {
                            quote!( #inner_ty )
                        } else {
                            quote!( Option<#inner_ty> )
                        }
                    }
                };

                let name = &arg.field_name;
                quote! {
                    let mut #name: #ty = ::core::default::Default::default();
                }
            }).chain(v.args.subcommand.as_ref().map(|cmd| {
                let name = &cmd.ident;
                let ty = &cmd.cmd_type;
                quote! {
                    let mut #name: Option<#ty> = ::core::option::Option::None;
                }
            }));
            let mut positional = 0;
            let mut match_args = v.args.all_args.iter().chain(v.args.synth_args.iter()).enumerate().map(|(argidx, arg)| {
                let field = &arg.field_name;
                let ty = &arg.inner_type;
                let processor = if arg.is_flag() {
                    match &arg.inverse_of {
                        Some(field) => {
                            // A field that uses `inverse_of` has type `()` and is normally never
                            // used by the program after parsing, so we inject a fake use of the
                            // field to silence the dead code warning.
                            alive_fields.push(field.clone());

                            let target = v.args.all_args.iter().find(|arg| arg.field_name == *field).unwrap();
                            let ty = &target.inner_type;
                            quote!( <#ty as #private::InvertibleFlag>::unset(&mut #field, __cx) )
                        }
                        None => quote!( <#ty as #private::FlagInternal>::set(&mut #field, __cx) ),
                    }
                } else {
                    quote! {{
                        let __value = (&&&&#private::Converter::<#ty>::new(__cx))
                            .convert(__raw)
                            .map_err(|e| __cx.err_value(e, #argidx))?;
                        __cx.try_insert(&mut #field, __value, #argidx)?;
                    }}.respan(ty.span()) // points trait errors to the field type
                };

                let mut matcher = TokenStream2::new();
                if let Some(long) = arg.long() {
                    matcher.extend(quote!(| #private::RawArg::Long(#long)));
                }
                if let Some(short) = arg.short() {
                    matcher.extend(quote!(| #private::RawArg::Short(#short)));
                }
                if arg.is_positional() {
                    let cmp = if arg.repeating {
                        quote!(>=)
                    } else {
                        quote!(==)
                    };
                    matcher.extend(
                        quote!(| #private::RawArg::Value(__raw) if __positional_idx #cmp #positional),
                    );
                    positional += 1;
                    quote! {
                        #matcher => {
                            __positional_idx += 1;
                            #processor
                        },
                    }
                } else {
                    let fetch_value = if arg.is_flag() {
                        quote!(__cx.no_value(__p, #argidx)?;)
                    } else {
                        quote!(let __raw = __cx.value(__p, #argidx)?;)
                    };
                    quote! {
                        #matcher => {
                            #fetch_value
                            #processor
                        },
                    }
                }
            }).collect::<Vec<_>>();

            if let Some(cmd) = &v.args.subcommand {
                let field = &cmd.ident;
                let ty = &cmd.cmd_type;
                match_args.push(quote! {
                    #private::RawArg::Value(__cmd) if __positional_idx >= #positional => {
                        #field = #private::Some(<#ty as #private::Subcommands>::dispatch(__cmd, __cx, __p)?);
                        break;
                    }
                });
            }

            let variant = &v.ident;
            let varname = if self.input.is_enum {
                quote!(#ident::#variant)
            } else {
                quote!(#ident)
            };
            let finalize = {
                let fields = v.args.all_args.iter().enumerate().map(|(arg_idx, arg)| {
                    let name = &arg.field_name;
                    let ty = &arg.inner_type;

                    if arg.required {
                        // If the argument is mandatory, but its intermediate value is `None`, raise an
                        // error.
                        if arg.wrapper_type == Some(WrapperType::Vec) {
                            quote! {
                                #name: if #name.is_empty() {
                                    return #private::Err(__cx.err_missing_arg(#arg_idx));
                                } else {
                                    #name
                                }
                            }
                        } else {
                            quote!( #name: #name.ok_or(__cx.err_missing_arg(#arg_idx))? )
                        }
                    } else if let Some(dfl) = &arg.default {
                        match dfl {
                            DefaultValue::Default => {
                                quote!( #name: #name.unwrap_or_else(#private::Default::default) )
                            },
                            DefaultValue::String(s) => {
                                // The specified string is a string literal and not parsed as an expression.
                                // This is to allow inclusion of the non-standard default value in generated
                                // `--help` output.

                                // Emit a unit test that ensures the specified string parses into the target type.
                                self.unit_test.extend(quote! {
                                    <#ty as #private::FromStr>::from_str(#s).unwrap();
                                });

                                quote! {
                                    #name: #name.unwrap_or_else(|| <#ty as #private::FromStr>::from_str(#s).unwrap())
                                }
                            },
                        }
                    } else {
                        quote!( #name )
                    }
                }).chain(v.args.subcommand.as_ref().map(|cmd| {
                    let name = &cmd.ident;
                    if cmd.is_optional {
                        quote!( #name )
                    } else {
                        quote!( #name: #name.ok_or(__cx.err_missing_subcommand())? )
                    }
                }));
                quote! {
                    __cx.version_or_help_request()?;

                    let __out = #varname {
                        #(#fields),*
                    };

                    // Mark all fields as used.
                    if let #varname { #(#alive_fields,)* .. } = &__out {
                        #(let _ = #alive_fields;)*
                    }

                    #private::Ok(__out)
                }
            };

            quote! {
                let mut __positional_idx = 0;
                #(#field_decls)*

                loop {
                    match __cx.next(__p)? {
                        #(#match_args)*
                        #private::RawArg::Eof => break,
                        unexpected => return #private::Err(__cx.err_unexpected(unexpected)),
                    }
                }

                #finalize
            }
        }).collect::<Vec<_>>();

        let mut subcommand_impl = TokenStream2::new();
        let dispatch = if self.input.is_enum {
            let subcommand_names = self.input.variants.iter().map(|v| {
                let s = syn::LitStr::new(&v.subcommand_name, v.ident.span());
                quote!(#s)
            });
            let subcommand_names_bytestr = self.input.variants.iter().map(|v| {
                let name = syn::LitByteStr::new(v.subcommand_name.as_bytes(), v.ident.span());
                quote!( #name )
            });
            let variant_desc = self
                .input
                .variants
                .iter()
                .map(|v| self.build_subcommand_desc(v))
                .collect::<Vec<_>>();

            let wildcard = match &self.input.fallback_variant {
                Some(var) => {
                    let name = &var.name;
                    quote!(_ => #private::Ok(Self::#name(__cx.drain(__cmd, __p))))
                }
                None => quote!(_ => #private::Err(__cx.err_unknown_subcommand(__cmd))),
            };

            let desc = self.build_enum_desc();
            subcommand_impl = quote! {
                impl #private::Subcommands for #ident {
                    const ENUM_DESC: #private::SubcommandEnumDesc = #desc;

                    fn dispatch(
                        __cmd: &#private::OsStr,
                        __cx: &#private::Context,
                        __p: &mut #private::ParseState,
                    ) -> #private::Result<Self> {
                        match __cmd.as_encoded_bytes() {
                            #( #subcommand_names_bytestr => {
                                let __cx = &__cx.for_subcommand(
                                    #subcommand_names,
                                    #private::command_desc(const { #variant_desc }),
                                );
                                #variant_parsers
                            }, )*
                            #wildcard
                        }
                    }
                }
            };

            // Read the first argument, which must be a subcommand name, and dispatch to that
            // variant.
            quote! {
                let cmd = match __cx.next(__p)? {
                    #private::RawArg::Value(cmd) => cmd,
                    arg => return #private::Err(__cx.err_missing_subcommand()),
                };
                <Self as #private::Subcommands>::dispatch(cmd, __cx, __p)
            }
        } else {
            // structs don't require initial dispatch, we can just create the storage for its only
            // variant directly.
            quote!( #(#variant_parsers)* )
        };

        let mut ancillary = TokenStream2::new();
        self.cx.attr_refs(private, |path| {
            ancillary.extend(quote! {
                let _ = #path;
            });
        });

        let unit_tests = if self.input.emit_tests && !self.unit_test.is_empty() {
            let content = &self.unit_test;
            let test_name = format_ident!("larpa_{}_tests", self.snake_case);
            quote! {
                #[test]
                fn #test_name() {
                    #content
                }
            }
        } else {
            TokenStream2::new()
        };
        Ok(quote! {
            const _: () = {
                // Imports traits needed for the autoref-based value conversion.
                // `prelude` has to reexport all traits using `Trait as _` to avoid name collisions.
                use #private::prelude::*;

                impl #private::CommandInternal for #ident {
                    fn parse(__cx: &#private::Context, __p: &mut #private::ParseState) -> #private::Result<Self> {
                        {
                            #ancillary
                        }

                        #dispatch
                    }
                }
                impl #private::Command for #ident {
                    const DESC: #private::CommandDesc = #desc;
                }
                #subcommand_impl
            };
            #unit_tests
        })
    }

    fn build_command_desc(&self) -> syn::Result<TokenStream2> {
        let Some(canonical_name) = self
            .input
            .canonical_name
            .as_deref()
            .or_else(|| self.meta.canonical_name())
        else {
            return Err(syn::Error::new(
                self.input.ident.span(),
                "`CARGO_BIN_NAME` and `CARGO_PKG_NAME` are unset; \
                 `#[larpa(name = \"…\")]` is required on this type",
            ));
        };

        let private = &self.private;
        let description = match self
            .input
            .description
            .as_deref()
            .or_else(|| self.meta.pkg_description.as_deref())
        {
            Some(desc) => &self.some(desc),
            None => self.none(),
        };
        let version = match &self.input.version {
            Some(v) => &self.some(v),
            None => match &self.meta.pkg_version {
                Some(v) => &self.some(v),
                None => self.none(),
            },
        };
        let authors = match &self.meta.pkg_authors {
            Some(auth) => &self.some(auth),
            None => self.none(),
        };
        let license = match &self.input.license {
            Some(lic) => &self.some(lic),
            None => self.none(),
        };
        let homepage = match &self.input.homepage {
            Some(v) => &self.some(v),
            None => self.none(),
        };
        let repository = match &self.input.repository {
            Some(v) => &self.some(v),
            None => self.none(),
        };

        let common = if self.input.is_enum {
            let version_formatter = match &self.input.version_fmt {
                Some(fmt) => fmt.to_token_stream(),
                None => quote!(#private::version::default_formatter),
            };

            quote! {
                version_formatter: #version_formatter,
                args: &[],
                subcommand_optional: false,
                subcommands: #private::Some(&<Self as #private::Subcommands>::ENUM_DESC),
            }
        } else {
            self.build_desc_common(&self.input.variants[0])
        };

        Ok(quote! {
            #private::command_desc(#private::CommandDescImpl(&#private::CommandDescInner {
                canonical_name: #canonical_name,
                description: #description,
                version: #version,
                authors: #authors,
                license: #license,
                homepage: #homepage,
                repository: #repository,
                #common
            }))
        })
    }

    fn build_enum_desc(&self) -> TokenStream2 {
        assert!(self.input.is_enum);

        let private = &self.private;
        let descs = self
            .input
            .variants
            .iter()
            .map(|v| self.build_subcommand_desc(v))
            .collect::<Vec<_>>();

        let has_fallback = self.input.fallback_variant.is_some();
        let discover_subcommands = if let Some(var) = &self.input.fallback_variant {
            match &var.discovery {
                Some(DiscoveryFn::Custom(path)) => quote!(#private::Some(#path)),
                Some(DiscoveryFn::Default) => {
                    quote!(#private::Some(#private::discover::discover_subcommands))
                }
                None => quote!(#private::None),
            }
        } else {
            quote!(#private::None)
        };

        quote! {
            #private::SubcommandEnumDesc {
                has_fallback: #has_fallback,
                discover_subcommands: #discover_subcommands,
                subcommands: &[ #(#private::subcommand_desc(#descs)),* ],
            }
        }
    }

    fn build_subcommand_desc(&self, variant: &Variant) -> TokenStream2 {
        let description = match variant.description.as_deref() {
            Some(desc) => &self.some(desc),
            None => self.none(),
        };

        let name = &variant.subcommand_name;

        let common = self.build_desc_common(variant);
        let private = &self.private;
        let none = &self.none;
        quote! {
            #private::CommandDescImpl(&#private::CommandDescInner {
                canonical_name: #name,
                description: #description,
                version: #none,
                authors: #none,
                license: #none,
                homepage: #none,
                repository: #none,
                #common
            })
        }
    }

    /// Builds the `CommandDescInner` fields common to subcommands and top-level commands.
    fn build_desc_common(&self, variant: &Variant) -> TokenStream2 {
        let args = variant
            .args
            .all_args
            .iter()
            .chain(variant.args.synth_args.iter())
            .map(|arg| self.build_arg_desc(arg))
            .collect::<Vec<_>>();

        let private = &self.private;
        let version_formatter = match &self.input.version_fmt {
            Some(fmt) => fmt.to_token_stream(),
            None => quote!(#private::version::default_formatter),
        };

        let mut subcommand_optional = false;
        if let VariantKind::Wrapped(ty) = &variant.kind {
            // Take the fields from the wrapped `Command`.
            return quote! {
                version_formatter: #version_formatter,
                args: const { #private::command_desc_impl(<#ty as #private::Command>::DESC).0.args },
                subcommand_optional: const { #private::command_desc_impl(<#ty as #private::Command>::DESC).0.subcommand_optional },
                subcommands: const { #private::command_desc_impl(<#ty as #private::Command>::DESC).0.subcommands },
            };
        }

        let subcommands = match &variant.args.subcommand {
            Some(SubcommandField {
                cmd_type,
                is_optional,
                ..
            }) => {
                subcommand_optional = *is_optional;
                quote! {
                    #private::Some(&<#cmd_type as #private::Subcommands>::ENUM_DESC)
                }
            }
            None => quote!(#private::None),
        };

        quote! {
            version_formatter: #version_formatter,
            args: &[ #(#args),* ],
            subcommand_optional: #subcommand_optional,
            subcommands: #subcommands,
        }
    }

    fn build_arg_desc(&self, arg: &Arg) -> TokenStream2 {
        let description = match arg.description.as_deref() {
            Some(desc) => &self.some(desc),
            None => self.none(),
        };
        let short = match arg.short() {
            Some(short) => &self.some(short),
            None => self.none(),
        };
        let long = match arg.long() {
            Some(long) => &self.some(long),
            None => self.none(),
        };
        let value_name = if arg.is_flag() {
            self.none()
        } else {
            &self.some(&arg.value_name)
        };
        let custom_default = if let Some(DefaultValue::String(s)) = &arg.default {
            &self.some(s)
        } else {
            self.none()
        };
        let optional = arg.is_optional();
        let repeating = arg.repeating;

        let private = &self.private;
        quote! {
            #private::argument_desc(#private::ArgumentDescImpl {
                description: #description,
                name: #private::argument_name(#private::ArgumentNameImpl {
                    short: #short,
                    long: #long,
                    value_name: #value_name,
                }),
                custom_default: #custom_default,
                optional: #optional,
                repeating: #repeating,
            })
        }
    }
}
