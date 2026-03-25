// proc_macro2::TokenStream — the library-friendly token stream type.
// Used instead of proc_macro::TokenStream so this logic can live in a
// normal library crate and be unit-tested without a compiler invocation.
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data::Struct,
    DataStruct, DeriveInput,
    Fields::Named,
    FieldsNamed,
    Ident,
    MetaList,       // Represents attribute syntax like:  exclude(foo, bar)
    Token,          // Macro that produces token types, e.g. Token![,] = Comma
    parse::{
        Parse,      // Trait: implement to make a type parseable from tokens
        ParseStream,// The cursor/input handed to Parse::parse()
        Parser,     // Trait that lets a function pointer act as a parser
    },
    parse2,         // Parses a TokenStream into any type that implements Parse
    punctuated::Punctuated, // A sequence separated by punctuation, e.g. a, b, c
};

// ── ExcludedFields ────────────────────────────────────────────────────────────
//
// Holds the list of field names that should NOT be made `pub` by this macro.
// Populated by parsing the attribute argument, e.g.:
//   #[make_public(exclude(secret, internal))]
//                 ^^^^^^^^^^^^^^^^^^^^^^^^^ — this part becomes ExcludedFields
struct ExcludedFields {
    fields: Vec<String>,
}

impl ExcludedFields {
    // Returns true if the given field ident appears in the exclusion list.
    //
    // `name` is `Option<Ident>` because syn models field names as optional
    // (tuple struct fields have no name). We treat unnamed fields as
    // not-excluded (unwrap_or(false)).
    fn contains(&self, name: &Option<Ident>) -> bool {
        name.as_ref()
            .map(|n| self.fields.iter().any(|f| f == &n.to_string()))
            .unwrap_or(false)
    }
}

// ── Parsing the attribute argument ───────────────────────────────────────────
//
// Implements syn's Parse trait so parse2() can turn the attribute token
// stream directly into an ExcludedFields value.
//
// Handles three cases:
//   (nothing)              → no exclusions, empty list
//   exclude(foo, bar)      → exclude fields foo and bar
//   anything else          → silently ignore, empty list
impl Parse for ExcludedFields {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Case 1: attribute was used with no arguments — #[make_public]
        // There's nothing to parse, return empty exclusion list.
        if input.is_empty() {
            return Ok(ExcludedFields { fields: vec![] });
        }

        // Case 2 & 3: try to parse the input as a MetaList.
        // MetaList covers the `path(tokens)` shape, e.g. `exclude(foo, bar)`.
        match input.parse::<MetaList>() {
            Ok(meta_list) => {
                // Only act if the path is specifically `exclude`.
                // Any other ident (e.g. `include(...)`) is silently ignored.
                if meta_list.path.is_ident("exclude") {
                    // Parse the contents of exclude(...) as a comma-separated
                    // list of identifiers: foo, bar, baz
                    //
                    // parse_terminated handles an optional trailing comma too.
                    // We pass it as a function pointer and call .parse2() on
                    // the inner token stream stored in meta_list.tokens.
                    let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
                    let idents = parser.parse2(meta_list.tokens).unwrap();

                    // Convert each Ident to a plain String for easy comparison
                    // later (avoids span-related inequality between idents).
                    let fields: Vec<String> = idents.iter().map(|v| v.to_string()).collect();
                    Ok(ExcludedFields { fields })
                } else {
                    // Unknown key — treat as no exclusions.
                    Ok(ExcludedFields { fields: vec![] })
                }
            }
            // Case 3: input wasn't parseable as MetaList at all.
            // Silently return empty rather than propagating an error,
            // making the macro more forgiving of unexpected attribute shapes.
            Err(_) => Ok(ExcludedFields { fields: vec![] }),
        }
    }
}

// ── Main proc-macro implementation ───────────────────────────────────────────
//
// This is an attribute macro (not a derive macro).
// Signature reflects that:
//   attr — tokens inside #[make_public(...)]
//   item — tokens of the entire struct the attribute is placed on
//
// What it does: re-emits the struct with all fields set to `pub`,
// EXCEPT fields listed in exclude(...), which keep their original visibility.
pub fn impl_(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the attribute argument into our ExcludedFields type.
    let excluded: ExcludedFields = parse2(attr).unwrap();

    // Parse the entire annotated item into syn's DeriveInput AST.
    // DeriveInput works for structs, enums, and unions — we'll restrict
    // to named structs below.
    let ast: DeriveInput = parse2(item).unwrap();

    // The struct's name, e.g. `MyStruct`.
    let name = &ast.ident;

    // Outer attributes on the struct itself, e.g. #[derive(Debug)].
    // We must re-emit these or they'd be lost — the macro replaces the
    // entire item, it doesn't patch it.
    let attrs = &ast.attrs;

    // Generic parameters: <T>, <T: Clone>, <'a>, etc.
    // Preserved verbatim so generic structs work correctly.
    let generics = &ast.generics;

    // ── Extract named fields ──────────────────────────────────────────────────
    //
    // Destructures through:
    //   ast.data → Data::Struct(DataStruct { fields, .. })
    //   fields   → Fields::Named(FieldsNamed { named, .. })
    //   named    → &Punctuated<Field, Comma>
    //
    // Panics for tuple structs, enums, unions — they're not supported.
    let fields = match &ast.data {
        Struct(DataStruct {
                   fields: Named(FieldsNamed { named, .. }),
                   ..
               }) => named,
        _ => unimplemented!("only works for structs with named fields"),
    };

    // ── Re-emit each field with adjusted visibility ───────────────────────────
    //
    // For every field we emit:
    //   - its own field-level attributes (e.g. #[serde(skip)]) unchanged
    //   - visibility: `pub` if not excluded, original vis if excluded
    //   - name and type unchanged
    //
    // Two branches keep the logic explicit and easy to read:
    let builder_fields = fields.iter().map(|f| {
        let field_name = &f.ident;
        let ty = &f.ty;
        let vis = &f.vis;           // original visibility (pub, pub(crate), private…)
        let field_attrs = &f.attrs; // field-level attributes like #[serde(rename = "x")]

        if excluded.contains(field_name) {
            // Excluded field: keep whatever visibility it had originally.
            // The author explicitly opted this field out of publicising.
            quote! { #(#field_attrs)* #vis #field_name: #ty }
        } else {
            // Normal field: force `pub` regardless of what it was before.
            // This is the whole point of the macro.
            quote! { #(#field_attrs)* pub #field_name: #ty }
        }
    });

    // ── Emit the final struct ─────────────────────────────────────────────────
    //
    // Reassembles the struct from its parts:
    //   #(#attrs)*        — re-emit all struct-level attributes
    //   pub struct #name  — always make the struct itself public
    //   #generics         — preserve generic parameters as-is
    //   #(#builder_fields,)* — expand the field iterator, comma-separated
    //
    // The entire original struct is replaced by this output — nothing
    // from `item` survives unless we explicitly re-emit it here.
    quote! {
        #(#attrs)*
        pub struct #name #generics {
            #(#builder_fields,)*
        }
    }
}