use proc_macro2::TokenStream;

// quote!        — turns Rust syntax written inline into a TokenStream
// format_ident! — creates an Ident (identifier token) from a formatted string
// quote_spanned!— like quote!, but attaches a specific source span so compiler
//                 errors point to the right location in user code
use quote::{format_ident, quote, quote_spanned};

// syn types used for parsing the input TokenStream into a typed AST:
// Attribute    — a single #[...] attribute
// Data::Struct — pattern-match variant confirming input is a struct
// DataStruct   — the struct's data payload
// DeriveInput  — top-level AST node for anything that #[derive(...)] is on
// Field        — one field inside a struct
// Fields::Named— pattern-match variant confirming fields have names (not tuple)
// FieldsNamed  — the collection of named fields
// Ident        — an identifier token (variable name, type name, etc.)
// LitStr       — a string literal token, e.g. "foo"
// Meta         — the content inside an attribute, e.g. the `rename("foo")` part
// parse2       — parses a proc_macro2::TokenStream into a syn type
// Spanned      — trait that lets you call .span() on AST nodes
use syn::{
    Attribute, Data::Struct, DataStruct, DeriveInput, Field, Fields::Named, FieldsNamed, Ident,
    LitStr, Meta, parse2, spanned::Spanned,
};

// ── HELPER: find a specific attribute on a field ─────────────────────────────
//
// Searches the field's attribute list for one whose path matches `name`.
// Returns a reference with the same lifetime as the field ('a) so the
// caller can borrow the attribute without cloning.
//
// Example: on `#[rename("foo")] pub bar: String`, calling
// extract_attribute_from_field(field, "rename") returns Some(&attr).
fn extract_attribute_from_field<'a>(f: &'a Field, name: &str) -> Option<&'a Attribute> {
    f.attrs.iter().find(|attr| attr.path().is_ident(name))
}

// ── HELPER: parse the new name out of #[rename("new_name")] ──────────────────
//
// Expects the attribute to be in Meta::List form — i.e. rename("foo"),
// not rename or rename = "foo". Parses the single string argument and
// turns it into an Ident so it can be used as a method name in quote!.
//
// Panics with a clear message if the attribute has the wrong shape,
// turning a cryptic syn error into an actionable one.
fn get_renamed_ident(attr: &Attribute) -> Ident {
    match &attr.meta {
        Meta::List(nested) => {
            // parse_args() re-parses the token contents of rename("foo")
            // as a LitStr — panics if it isn't a plain string literal.
            let s: LitStr = nested.parse_args().unwrap();
            // Build an Ident from the string value, preserving the span
            // of the literal so errors still point at the right place.
            Ident::new(&s.value(), s.span())
        }
        _ => panic!("expected #[rename(\"name\")]"),
    }
}

// ── HELPER: check if #[builder_defaults] is present on the struct ─────────────
//
// When present, the generated build() method calls .unwrap_or_default()
// instead of .expect(...), and compile-time assertions verify that every
// field type implements Default.
fn use_defaults(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("builder_defaults"))
}

// ── MAIN ENTRY POINT ──────────────────────────────────────────────────────────
//
// Called by the proc-macro crate's #[proc_macro_derive] entry point.
// Receives the TokenStream of the item #[derive(Builder)] was placed on,
// and returns a TokenStream of new code to append to the crate.
pub fn impl_(input: TokenStream) -> TokenStream {
    // Parse the raw token stream into a structured AST we can inspect.
    let ast: DeriveInput = parse2(input).unwrap();

    // The name of the struct the derive is on, e.g. `MyStruct`.
    let name = &ast.ident;

    // The name we'll give the generated builder, e.g. `MyStructBuilder`.
    let builder_name = format_ident!("{}Builder", name);

    // Check for #[builder_defaults] on the struct itself (not its fields).
    let defaults = use_defaults(&ast.attrs);

    // ── Extract the named fields ──────────────────────────────────────────────
    //
    // We only support structs with named fields (`struct Foo { bar: T }`).
    // The nested pattern destructures through:
    //   ast.data           → Data::Struct(DataStruct { fields, .. })
    //   fields             → Fields::Named(FieldsNamed { named, .. })
    //   named              → &Punctuated<Field, Comma>  ← what we want
    //
    // Any other shape (tuple struct, enum, union) hits the unimplemented!.
    let fields = match &ast.data {
        Struct(DataStruct {
                   fields: Named(FieldsNamed { named, .. }),
                   ..
               }) => named,
        _ => unimplemented!("only works for structs with named fields"),
    };

    // ── 1. Builder struct field declarations ─────────────────────────────────
    //
    // For each field `foo: T` in the original struct, emit:
    //   foo: std::option::Option<T>
    //
    // Wrapping in Option lets us distinguish "not yet set" from any value,
    // and using the full path avoids collisions if the user shadows `Option`.
    let builder_fields = fields.iter().map(|f| {
        let field_name = &f.ident;
        let ty = &f.ty;
        quote! { #field_name: std::option::Option<#ty> }
    });

    // ── 2. Builder::default() field initializers ──────────────────────────────
    //
    // For each field, emit:
    //   foo: std::option::Option::None
    //
    // Used in `MyStruct::builder()` to construct a blank builder where
    // nothing has been set yet.
    let builder_defaults = fields.iter().map(|f| {
        let field_name = &f.ident;
        quote! { #field_name: std::option::Option::None }
    });

    // ── 3. Setter methods ─────────────────────────────────────────────────────
    //
    // For each field, emit a consuming setter:
    //   pub fn foo(mut self, input: T) -> Self {
    //       self.foo = Some(input);
    //       self
    //   }
    //
    // If the field has #[rename("bar")], the method is named `bar` instead
    // of `foo`, while the struct field assignment still uses the real name.
    // Consuming `self` (not `&mut self`) enables method chaining:
    //   MyStruct::builder().foo(1).bar("x").build()
    let setter_methods = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let ty = &f.ty;

        // Resolve the public method name — either the rename or the field name.
        let method_name = match extract_attribute_from_field(f, "rename") {
            Some(attr) => get_renamed_ident(attr),
            None => field_name.clone(),
        };

        quote! {
            pub fn #method_name(mut self, input: #ty) -> Self {
                self.#field_name = std::option::Option::Some(input);
                self
            }
        }
    });

    // ── 4. build() field assignments ─────────────────────────────────────────
    //
    // For each field, emit the expression that extracts the final value
    // from Option<T> when build() is called.
    //
    // With #[builder_defaults]:
    //   foo: self.foo.unwrap_or_default()
    //   → uses Default::default() if the setter was never called.
    //
    // Without #[builder_defaults]:
    //   foo: self.foo.expect("field not set: foo")
    //   → panics at runtime with a clear message if the setter was skipped.
    let build_fields = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let name_str = field_name.to_string();
        if defaults {
            quote! { #field_name: self.#field_name.unwrap_or_default() }
        } else {
            quote! {
                #field_name: self.#field_name
                    .expect(concat!("field not set: ", #name_str))
            }
        }
    });

    // ── 5. Compile-time Default assertions ───────────────────────────────────
    //
    // Only generated when #[builder_defaults] is present.
    //
    // For each field of type T, emits a zero-sized struct with a where clause:
    //   struct __fooDefaultAssertion where T: core::default::Default;
    //
    // This is a compile-time trick: the struct itself is never used, but the
    // where clause forces the compiler to verify T: Default. If it doesn't,
    // the error points at the field's type in the *user's* source file
    // (thanks to quote_spanned! using ty.span()), not inside the macro.
    //
    // Without this, unwrap_or_default() would also fail, but with a much
    // harder-to-read error message deep inside generated code.
    let default_asserts: Vec<TokenStream> = if defaults {
        fields
            .iter()
            .map(|f| {
                let field_name = f.ident.as_ref().unwrap();
                let ty = &f.ty;
                // Unique name per field to avoid duplicate struct definitions.
                let assertion_ident = format_ident!("__{}DefaultAssertion", field_name);

                // quote_spanned! pins the error to ty's location in user code.
                quote_spanned! { ty.span() =>
                    #[allow(dead_code, non_camel_case_types)]
                    struct #assertion_ident where #ty: core::default::Default;
                }
            })
            .collect()
    } else {
        vec![]
    };

    // ── Final code generation ─────────────────────────────────────────────────
    //
    // Assembles all the pieces into the three items that get appended to the
    // user's crate:
    //
    // 1. `pub struct MyStructBuilder { foo: Option<T>, ... }`
    //    The builder type with one Option field per original field.
    //
    // 2. `impl MyStruct { pub fn builder() -> MyStructBuilder { ... } }`
    //    A static constructor that returns a blank builder.
    //
    // 3. `impl MyStructBuilder { fn foo(...) setters + fn build(...) }`
    //    All setters and the final build() method.
    //    The #(#default_asserts)* expands to nothing when defaults is false.
    //
    // The #(#iter,)* syntax is quote!'s repetition: it expands each item in
    // the iterator separated by commas (or nothing for #(#iter)*).
    quote! {
        pub struct #builder_name {
            #(#builder_fields,)*
        }

        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#builder_defaults,)*
                }
            }
        }

        impl #builder_name {
            #(#setter_methods)*

            pub fn build(self) -> #name {
                // Zero-sized assertion structs — compiled away, errors stay
                // pointing at user code.
                #(#default_asserts)*

                #name {
                    #(#build_fields,)*
                }
            }
        }
    }
}