mod builder;
mod greet;
mod private;
mod public;
mod resource;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse2, DeriveInput};

#[proc_macro_derive(Greet)]
pub fn derive_greet(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse2(input.into()).unwrap();
    let name = &ast.ident;

    quote! {
        impl #name {
            pub fn greet(&self) -> String {
                format!("Hello from {}!", stringify!(#name))
            }
        }
    }.into()
}

#[proc_macro_derive(Info)]
pub fn derive_info(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse2(input.into()).unwrap();
    let name = &ast.ident;

    quote! {
        impl Info for #name {
           fn info() -> String {
                format!("Hello {}!", stringify!(#name))
            }
        }
    }.into()
}

#[proc_macro_attribute]
pub fn public(attr: TokenStream, item: TokenStream) -> TokenStream {
    public::impl_(attr.into(), item.into()).into()
}

#[proc_macro]
pub fn private(item: TokenStream) -> TokenStream {
    private::impl_(item.into()).into()
}

#[proc_macro_derive(Builder, attributes(rename, builder_defaults))]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    builder::impl_(input.into()).into()
}

#[proc_macro]
pub fn resource(item: TokenStream) -> TokenStream {
    resource::impl_(item.into()).into()
}
