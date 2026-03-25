use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse2};

pub fn impl_(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse2(input).unwrap();
    let name = &ast.ident;

    quote! {
        impl #name {
            pub fn greet(&self) -> String {
                format!("Hello from {}!", stringify!(#name))
            }
        }
    }
}
