mod attr;
mod codegen;
mod input;
mod meta;
mod name;
mod respan;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use crate::{attr::Context, codegen::Codegen, input::Input, meta::Metadata};

/// A `#[derive]` macro for larpa's `Command` trait.
#[proc_macro_derive(Command, attributes(larpa))]
pub fn derive_command(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut cx = Context::default();
    match derive_command_impl(&mut cx, input) {
        Ok(ts) => ts.into(),
        Err(error) => {
            let krate = cx.crate_path().unwrap_or_else(default_crate_path);
            let private = quote!(#krate::private);
            let mut attr_paths = TokenStream2::new();
            cx.attr_refs(&private, |path| {
                attr_paths.extend(quote!( let _ = #path; ));
            });

            let ide_helper = quote! {
                const _: () = {
                    #attr_paths
                };
            };

            let error = error.to_compile_error();

            quote! {
                #ide_helper
                #error
            }
            .into()
        }
    }
}

fn derive_command_impl(cx: &mut Context, input: DeriveInput) -> syn::Result<TokenStream2> {
    let input = Input::parse(cx, &input)?;
    let meta = Metadata::get();

    let mut cg = Codegen::new(cx, &input, &meta);
    cg.derive()
}

fn default_crate_path() -> syn::Path {
    syn::parse_str("::larpa").unwrap()
}
