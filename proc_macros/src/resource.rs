use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident,   // An identifier token — used here for the struct name
    LitInt,  // An integer literal token, e.g. `42`
    LitStr,  // A string literal token, e.g. `"hello"`
    parse::{Parse, ParseStream},
    parse2,
};

// ── Custom keywords ───────────────────────────────────────────────────────────
//
// syn::custom_keyword! generates a new type for each word that acts as a
// parser for that exact identifier. For example, `kw::resource` will only
// match the token `resource` — not any other identifier.
//
// Without this, syn would parse `resource`, `name`, `count` as generic
// Ident tokens, and you'd have to manually check their string values.
// Custom keywords give you type-safe parsing AND better error messages.
//
// They live in a module to avoid polluting the crate namespace — callers
// write `kw::name` rather than just `name`, which would clash with
// Rust's built-in concepts.
//
// Usage example:
//   input.parse::<kw::resource>()?  — consumes `resource` or errors
//   input.peek(kw::name)            — checks next token without consuming
mod kw {
    syn::custom_keyword!(resource); // matches the token: resource
    syn::custom_keyword!(name);     // matches the token: name
    syn::custom_keyword!(count);    // matches the token: count
}

// ── Parsed representation of the macro input ─────────────────────────────────
//
// Holds the three pieces of information extracted from input like:
//   resource MyPool name "database" count 10
//   ^^^^^^^  ^^^^^^ ^^^^ ^^^^^^^^^^ ^^^^^ ^^
//   keyword  ident  key  value      key   value
struct ResourceInput {
    struct_name: Ident,   // the identifier after `resource`
    name_value: String,   // the string after `name`
    count_value: u32,     // the integer after `count`
}

// ── Custom parser for the macro's DSL ────────────────────────────────────────
//
// Teaches syn how to turn a token stream into a ResourceInput.
// The DSL grammar is:
//
//   resource <Ident> (<name_kw> <LitStr> | <count_kw> <LitInt>)*
//
// `name` and `count` are optional, can appear in any order, and can be
// repeated (last write wins). This is intentional flexibility — the macro
// doesn't enforce presence or ordering.
impl Parse for ResourceInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Mandatory: input must start with the `resource` keyword.
        // parse::<kw::resource>() consumes it if present, or returns an
        // error like "expected `resource`" if something else is there.
        input.parse::<kw::resource>()?;

        // Mandatory: the struct name immediately follows `resource`.
        // Parsed as a plain Ident — any valid Rust identifier works.
        let struct_name: Ident = input.parse()?;

        // Defaults for optional fields.
        // If neither `name` nor `count` appear, these values are used.
        let mut name_value = String::new();
        let mut count_value = 0u32;

        // ── Key-value loop ────────────────────────────────────────────────────
        //
        // Consume the rest of the input as zero or more key-value pairs.
        // peek() looks at the next token WITHOUT consuming it — this lets
        // us decide which branch to take before committing to a parse.
        // This is the standard syn pattern for optional/ordered fields.
        while !input.is_empty() {
            if input.peek(kw::name) {
                // Consume the `name` keyword (we already know it's there).
                input.parse::<kw::name>()?;
                // Parse the value — must be a string literal like "foo".
                // .value() strips the surrounding quotes, giving a String.
                let lit: LitStr = input.parse()?;
                name_value = lit.value();

            } else if input.peek(kw::count) {
                // Consume the `count` keyword.
                input.parse::<kw::count>()?;
                // Parse the value — must be an integer literal like 42.
                // base10_parse() converts the token's text to u32,
                // returning a syn::Result so overflow is handled cleanly.
                let lit: LitInt = input.parse()?;
                count_value = lit.base10_parse()?;

            } else {
                // Unknown token — nothing left we know how to handle.
                // input.error() creates a syn::Error pointing at the
                // current position, so the compiler highlights exactly
                // where the problem is in the user's macro invocation.
                return Err(input.error("expected `name` or `count`"));
            }
        }

        Ok(ResourceInput {
            struct_name,
            name_value,
            count_value,
        })
    }
}

// ── Code generation ───────────────────────────────────────────────────────────
//
// Takes the parsed DSL input and generates two items:
//   1. A struct definition with `name` and `count` fields
//   2. An impl block with `new()` and `description()` methods
//
// Example input:   resource MyPool name "db" count 5
// Example output:
//   struct MyPool { pub name: &'static str, pub count: u32 }
//   impl MyPool {
//       pub fn new() -> Self { Self { name: "db", count: 5 } }
//       pub fn description(&self) -> String { ... }
//   }
pub fn impl_(input: TokenStream) -> TokenStream {
    let ri: ResourceInput = parse2(input).unwrap();

    // Pull values out of the parsed struct for use inside quote!.
    // quote! can interpolate references directly with #variable syntax.
    let struct_name = &ri.struct_name;
    let name_value = &ri.name_value;   // &String — quote! emits it as a str literal
    let count_value = ri.count_value;  // u32 — copied, not referenced (Copy type)

    quote! {
        // ── Generated struct ──────────────────────────────────────────────────
        //
        // Fields are always &'static str and u32 regardless of what the
        // user passed — the DSL defines a fixed shape, not a generic one.
        // Note: `pub` on fields inside a non-pub struct still works —
        // the struct's own visibility controls external access.
        struct #struct_name {
            pub name: &'static str,
            pub count: u32,
        }

        impl #struct_name {
            // ── Constructor ───────────────────────────────────────────────────
            //
            // Bakes the DSL values directly into the generated code as
            // literals — there's no runtime parsing or configuration.
            // #name_value  → emitted as a string literal e.g. "database"
            // #count_value → emitted as an integer literal e.g. 10
            pub fn new() -> Self {
                Self {
                    name: #name_value,
                    count: #count_value,
                }
            }

            // ── Description method ────────────────────────────────────────────
            //
            // stringify!(#struct_name) is a standard Rust macro that turns
            // an identifier into its string representation AT COMPILE TIME.
            // It runs in the context of the generated code, not here in the
            // proc macro — so it sees the actual struct name token.
            //
            // Using std::string::String (full path) avoids any ambiguity if
            // the user has a `use` that shadows `String` in their crate.
            pub fn description(&self) -> std::string::String {
                format!("{}: {} (count: {})",
                    stringify!(#struct_name), self.name, self.count)
            }
        }
    }
}