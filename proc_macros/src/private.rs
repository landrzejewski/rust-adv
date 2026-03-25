// proc_macro2::TokenStream — library-safe token stream.
// We need it twice: once to parse into an AST, once to re-emit the
// original struct unchanged. That's why we clone it before consuming it.
use proc_macro2::TokenStream;

// format_ident! — creates an Ident from a format string, e.g.
//   format_ident!("get_{}", "foo")  →  get_foo
// quote!        — turns inline Rust syntax into a TokenStream
use quote::{format_ident, quote};

use syn::{
    Data::Struct, DataStruct, DeriveInput,
    Fields::Named, FieldsNamed,
    parse2,
};

// ── Main entry point ──────────────────────────────────────────────────────────
//
// This is a function-like proc macro:
//   private!(struct Foo { bar: String, baz: i32 })
//
// Generates the struct with private fields plus a getter for each field:
//   impl Foo {
//       pub fn get_bar(&self) -> &String { &self.bar }
//       pub fn get_baz(&self) -> &i32    { &self.baz  }
//   }
//
// Function-like proc macros replace their input entirely (like attribute
// macros). To keep the original struct, we must re-emit it ourselves —
// hence the clone before parsing.
pub fn impl_(input: TokenStream) -> TokenStream {
    // Clone BEFORE parse2() consumes `input`.
    // We need the original tokens to re-emit the struct verbatim later.
    // parse2() takes ownership, so without this clone we'd lose the
    // original struct definition.
    let item_clone = input.clone();

    // Parse the token stream into syn's DeriveInput — the universal AST
    // node for anything a derive macro can be placed on (struct/enum/union).
    let ast: DeriveInput = parse2(input).unwrap();

    // The struct's name as an Ident, e.g. `Foo`.
    // Used in `impl #name { ... }` below.
    let name = &ast.ident;

    // ── Extract named fields ──────────────────────────────────────────────────
    //
    // Drill through the AST layers:
    //   ast.data  →  Data::Struct(DataStruct { fields, .. })
    //   fields    →  Fields::Named(FieldsNamed { named, .. })
    //   named     →  &Punctuated<Field, Comma>  ← the actual field list
    //
    // The double-nested pattern match handles all the wrapping syn uses
    // to represent the difference between:
    //   struct Foo { x: i32 }      ← Named   (what we want)
    //   struct Foo(i32)            ← Unnamed  (rejected)
    //   struct Foo;                ← Unit     (rejected)
    let fields = match &ast.data {
        Struct(DataStruct {
                   fields: Named(FieldsNamed { named, .. }),
                   ..
               }) => named,
        _ => unimplemented!("only works for structs with named fields"),
    };

    // ── Generate one getter method per field ──────────────────────────────────
    //
    // Collecting into Vec<TokenStream> (rather than keeping a lazy iterator)
    // because quote!'s #(#methods)* repetition needs the iterator to be
    // re-usable — a consumed iterator would produce nothing.
    let methods: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            // .as_ref().unwrap() — ident is Option<Ident> because tuple struct
            // fields have no name. Safe to unwrap here since we already
            // verified above that all fields are named.
            let field_name = f.ident.as_ref().unwrap();

            // The field's type exactly as written, e.g. `String`, `Vec<i32>`.
            // We return &T (shared reference) so the getter doesn't move
            // or clone the value.
            let type_name = &f.ty;

            // Build the method name: `bar` → `get_bar`.
            // format_ident! preserves span info better than string
            // manipulation — errors point at the right place.
            let method_name = format_ident!("get_{}", field_name);

            // Emit the getter:
            //   pub fn get_bar(&self) -> &String {
            //       &self.bar
            //   }
            //
            // &self      — immutable borrow, no ownership transfer
            // -> &#type_name — returns a reference to the field's type
            // &self.#field_name — borrows the field directly, lifetime
            //                     tied to &self automatically by the compiler
            quote! {
                pub fn #method_name(&self) -> &#type_name {
                    &self.#field_name
                }
            }
        })
        .collect();

    // ── Final output ──────────────────────────────────────────────────────────
    //
    // Two items are emitted back to the compiler, one after the other:
    //
    // 1. #item_clone
    //    The original struct definition, re-emitted verbatim.
    //    Required because this is a function-like proc macro (#[proc_macro]),
    //    which replaces its input. Unlike #[proc_macro_derive], the compiler
    //    does not automatically keep the original item.
    //
    // 2. impl #name { #(#methods)* }
    //    The generated impl block with all getters.
    //    #(#methods)* expands each TokenStream in the Vec one after another
    //    with no separator (methods don't need commas between them).

    quote! {
        #item_clone

        impl #name {
            #(#methods)*
        }
    }
}