#![allow(
    unused_macros,
    unused_imports,
    unused_mut,
    dead_code,
    unused_variables,
    unreachable_patterns
)]

use std::collections::HashMap;

use proc_macros::{Builder, Greet, private, public, resource};

// ===========================================================================
// Section 0: Macro Theory
// ===========================================================================

fn macro_theory() {
    println!("\n=== Section 0: Macro Theory ===\n");

    println!("Macros are Rust's metaprogramming facility — code that writes code.");
    println!("They run at compile time, generating source code that the compiler");
    println!("then type-checks and compiles like any other code.\n");

    println!("Rust has two macro families:");
    println!("  1. Declarative macros (macro_rules!) — pattern-match on token");
    println!("     sequences and expand to code. Most common for everyday use.");
    println!("  2. Procedural macros — Rust functions that receive and return");
    println!("     token streams. Three kinds:");
    println!("       - Derive macros:        #[derive(Name)]");
    println!("       - Attribute macros:     #[name] or #[name(...)]");
    println!("       - Function-like macros: name!(...)\n");

    println!("Why macros exist (things functions cannot do):");
    println!("  - Accept a variable number of arguments (println! takes 0..N)");
    println!("  - Generate struct/enum/fn definitions at compile time");
    println!("  - Create domain-specific languages (DSLs) with custom syntax");
    println!("  - Reduce boilerplate: derive impls, builder patterns, routing\n");

    println!("How they differ from functions:");
    println!("  - Operate on syntax tokens, not runtime values");
    println!("  - Expand before type checking — can generate any syntactic construct");
    println!("  - Cannot be stored in variables or passed as arguments");
    println!("  - Invoked with ! (e.g., println!, vec!)\n");

    println!("When to use macros:");
    println!("  - DO: variadic args, boilerplate reduction, DSLs, conditional codegen");
    println!("  - DON'T: when a function or generic covers the use case —");
    println!("    macros are harder to read, debug, and maintain");
}

// ===========================================================================
// Section 1: Declarative Basics
// ===========================================================================

fn declarative_basics() {
    println!("\n=== Section 1: Declarative Basics ===\n");

    // --- Step 1: Simplest macro ---
    // A macro with no arguments — just expands to fixed code
    macro_rules! hello {
        () => {
            println!("  Hello from a macro!")
        };
    }

    hello!();

    // --- Step 2: Capturing expressions ---
    // $x:expr captures any Rust expression and stringify! shows the source
    macro_rules! show {
        ($x:expr) => {
            println!("  show!: {} = {}", stringify!($x), $x)
        };
    }

    show!(1 + 2);
    show!("hello".len());
    show!(vec![1, 2, 3].iter().sum::<i32>());
    show!({
        let a = 10;
        a * a
    });

    // --- Step 3: Bracket styles ---
    // All three bracket styles are equivalent to the compiler
    macro_rules! greet {
        ($name:expr) => {
            println!("  Hi, {}!", $name)
        };
    }

    greet!("parentheses"); // () — function-like convention
    greet!["brackets"]; // [] — collection-like convention (vec![])
    greet! { "braces" }; // {} — block-like convention

    // Convention: () for function calls, [] for collections,
    // {} for item-defining macros (macro_rules! name { ... })

    // --- Step 4: Generating items ---
    // $name:ident captures identifiers — can create struct/fn definitions
    macro_rules! make_unit_struct {
        ($name:ident) => {
            #[derive(Debug)]
            struct $name;
        };
    }

    make_unit_struct!(Marker);
    make_unit_struct!(Tag);
    make_unit_struct!(Sentinel);
    println!(
        "  Generated structs: {:?}, {:?}, {:?}",
        Marker, Tag, Sentinel
    );

    // Macros can also generate functions
    macro_rules! make_getter {
        ($name:ident, $val:expr) => {
            fn $name() -> i32 {
                $val
            }
        };
    }

    make_getter!(get_answer, 42);
    make_getter!(get_zero, 0);
    println!(
        "  Generated fns: get_answer()={}, get_zero()={}",
        get_answer(),
        get_zero()
    );
}

// ===========================================================================
// Section 2: Fragment Specifiers
// ===========================================================================

fn fragment_specifiers() {
    println!("\n=== Section 2: Fragment Specifiers ===\n");

    // --- Step 5: Common specifiers: ident, ty, expr ---
    // ident = identifier, ty = type, expr = expression
    macro_rules! make_var {
        ($name:ident, $ty:ty, $val:expr) => {
            let $name: $ty = $val;
        };
    }

    make_var!(x, i32, 42);
    make_var!(greeting, String, "hello".to_string());
    make_var!(nums, Vec<i32>, vec![1, 2, 3]);
    println!("  make_var!: x={x}, greeting={greeting}, nums={nums:?}");

    // --- Step 6: block and literal ---
    // block captures { ... }, literal captures only literal values
    macro_rules! run_block {
        ($b:block) => {
            println!("  run_block! result: {}", $b)
        };
    }

    run_block!({ 2 + 3 });
    run_block!({
        let a = 10;
        let b = 20;
        a + b
    });

    macro_rules! double_literal {
        ($l:literal) => {
            println!("  double_literal!: {} * 2 = {}", $l, $l * 2)
        };
    }

    double_literal!(21);
    double_literal!(100);
    // double_literal!(1 + 2);  // Won't compile: 1 + 2 is an expr, not a literal

    // --- Step 7: tt — the wildcard ---
    // tt matches any single token tree: a token or a delimited group
    macro_rules! what_is {
        ($t:tt) => {
            println!("  what_is!: `{}`", stringify!($t))
        };
    }

    what_is!(hello); // single ident token
    what_is!(42); // single literal token
    what_is!(+); // single punct token
    what_is!((a, b, c)); // delimited group (parens) = one tt
    what_is!([1, 2]); // delimited group (brackets) = one tt

    // A delimited group counts as ONE tt regardless of contents
    macro_rules! count_args {
        ($($t:tt)*) => { 0usize $(+ { let _ = stringify!($t); 1 })* };
    }

    println!("  count_args!(a b c): {}", count_args!(a b c));
    println!("  count_args!((a b c)): {}", count_args!((a b c)));

    // --- Step 8: Advanced specifiers: vis, pat, lifetime ---
    // vis captures visibility qualifiers (pub, pub(crate), or empty)
    macro_rules! make_field_struct {
        ($sname:ident { $vis:vis $fname:ident : $ty:ty }) => {
            #[derive(Debug)]
            struct $sname { $vis $fname: $ty }
        };
    }

    make_field_struct!(PublicField { pub value: i32 });
    make_field_struct!(PrivateField { count: u32 });
    let pf = PublicField { value: 99 };
    let pvf = PrivateField { count: 5 };
    println!("  vis specifier: {:?}, {:?}", pf, pvf);

    // pat captures patterns (for use in match arms, let bindings)
    macro_rules! match_pattern {
        ($val:expr, $pat:pat => $result:expr) => {
            match $val {
                $pat => println!("  pat specifier: matched → {}", $result),
                _ => println!("  pat specifier: no match"),
            }
        };
    }

    match_pattern!(Some(42), Some(x) => x);
    match_pattern!((1, 2), (a, b) => a + b);

    // lifetime captures lifetime parameters
    macro_rules! print_lifetime {
        ($lt:lifetime) => {
            println!("  lifetime specifier: {}", stringify!($lt))
        };
    }

    print_lifetime!('a);
    print_lifetime!('static);
}

// ===========================================================================
// Section 3: Multiple Arms and Repetition
// ===========================================================================

fn arms_and_repetition() {
    println!("\n=== Section 3: Multiple Arms and Repetition ===\n");

    // --- Step 9: Multiple match arms ---
    // First matching arm wins (like match expressions)
    macro_rules! say {
        () => {
            println!("  say!: (silence)")
        };
        ($x:expr) => {
            println!("  say!: {}", $x)
        };
        ($x:expr, $y:expr) => {
            println!("  say!: {} and {}", $x, $y)
        };
    }

    say!();
    say!("hello");
    say!("hello", "world");

    // First-match-wins: more specific arms must come before general ones
    macro_rules! classify {
        (0) => {
            "zero"
        };
        (1) => {
            "one"
        };
        ($x:expr) => {
            "other"
        };
    }

    println!(
        "  classify!(0)={}, classify!(1)={}, classify!(42)={}",
        classify!(0),
        classify!(1),
        classify!(42)
    );

    // --- Step 10: Zero-or-more * ---
    // $(...),* matches zero or more comma-separated items
    macro_rules! sum {
        ($($x:expr),*) => {{
            let mut total = 0;
            $(total += $x;)*
            total
        }};
    }

    println!("  sum!(): {}", sum!());
    println!("  sum!(1, 2, 3): {}", sum!(1, 2, 3));
    println!("  sum!(10, 20, 30, 40): {}", sum!(10, 20, 30, 40));

    // --- Step 11: One-or-more + and trailing comma ---
    // + requires at least one match; $(,)? allows optional trailing comma
    macro_rules! product {
        ($($x:expr),+ $(,)?) => {{
            let mut result = 1i64;
            $(result *= $x;)+
            result
        }};
    }

    println!("  product!(2, 3, 4): {}", product!(2, 3, 4));
    println!("  product!(10,): {}", product!(10,));
    // product!();  // Won't compile: + requires at least one argument

    macro_rules! first {
        ($head:expr $(, $rest:expr)* $(,)?) => {
            $head
        };
    }

    println!("  first!(1, 2, 3): {}", first!(1, 2, 3));
    println!("  first!(42,): {}", first!(42,));

    // --- Step 12: hashmap! macro ---
    // Practical example combining repetition with trailing comma
    macro_rules! hashmap {
        ($($key:expr => $val:expr),* $(,)?) => {{
            let mut map = HashMap::new();
            $(map.insert($key, $val);)*
            map
        }};
    }

    let scores = hashmap! {
        "alice" => 100,
        "bob" => 85,
        "carol" => 92,
    };
    println!("  hashmap!: {:?}", scores);

    let empty: HashMap<&str, i32> = hashmap!();
    println!("  hashmap!() empty: {:?}", empty);

    // --- Step 13: Optional parts with ? ---
    // $(...)? matches zero or one occurrence
    macro_rules! make_fn {
        ($name:ident () $(-> $ret:ty)? $body:block) => {
            fn $name() $(-> $ret)? $body
        };
    }

    make_fn!(get_five() -> i32 { 5 });
    make_fn!(say_hi() { println!("  make_fn!: hi from generated function") });

    println!("  make_fn! with return: {}", get_five());
    say_hi();

    // --- Step 14: Nested repetition ---
    // Two levels of $() for processing groups of items
    macro_rules! matrix {
        ($([$($val:expr),* $(,)?]),* $(,)?) => {
            vec![$(vec![$($val),*]),*]
        };
    }

    let m = matrix![[1, 2, 3], [4, 5, 6], [7, 8, 9],];
    println!("  matrix!: {:?}", m);

    // Nested repetition: declare multiple enums
    macro_rules! declare_enums {
        ($(enum $name:ident { $($variant:ident),* $(,)? })*) => {
            $(
                #[derive(Debug)]
                enum $name { $($variant),* }
            )*
        };
    }

    declare_enums! {
        enum Light { Red, Yellow, Green }
        enum Size { Small, Medium, Large }
    }
    println!("  declare_enums!: {:?}, {:?}", Light::Green, Size::Large);
}

// ===========================================================================
// Section 4: Macro Hygiene
// ===========================================================================

fn macro_hygiene() {
    println!("\n=== Section 4: Macro Hygiene ===\n");

    // --- Step 15: Local variables don't leak ---
    // Variables created inside a macro are invisible outside (hygiene)
    macro_rules! create_x {
        () => {
            let x = 42;
            println!("  inside create_x!: x = {x}");
        };
    }

    create_x!();
    // println!("{x}");  // Error: x is not in scope here
    // The macro's x lives in a different syntactic context

    let x = 99;
    println!("  outside create_x!: our x = {x}");

    // Variables CAN be shared if the caller provides the name
    macro_rules! set_var {
        ($name:ident, $val:expr) => {
            let $name = $val;
        };
    }

    set_var!(y, 123);
    println!("  set_var!(y, 123): y = {y}");

    // --- Step 16: Items DO leak ---
    // Structs, functions, impls generated by macros ARE visible
    macro_rules! make_greeter {
        () => {
            struct AutoGreeter;
            impl AutoGreeter {
                fn greet(&self) -> &str {
                    "Hello from macro-generated struct!"
                }
            }
        };
    }

    make_greeter!();
    let g = AutoGreeter;
    println!("  items leak: {}", g.greet());

    // --- Step 17: stringify! and concat! ---
    // stringify! converts tokens to &str without evaluating
    // concat! joins literals into a single &str at compile time
    println!("  stringify!(1 + 2) = \"{}\"", stringify!(1 + 2));
    println!("  stringify!(Vec<i32>) = \"{}\"", stringify!(Vec<i32>));
    println!(
        "  concat!(\"hello\", \" \", \"world\") = \"{}\"",
        concat!("hello", " ", "world")
    );
    println!(
        "  concat!(\"v\", 1, '.', 0) = \"{}\"",
        concat!("v", 1, '.', 0)
    );

    macro_rules! assert_positive {
        ($val:expr) => {
            assert!(
                $val > 0,
                "{} must be positive, got {}",
                stringify!($val),
                $val
            );
            println!(
                "  assert_positive!({}): OK (value = {})",
                stringify!($val),
                $val
            );
        };
    }

    assert_positive!(5);
    assert_positive!(1 + 2 + 3);
    // assert_positive!(-1);  // Would panic: "-1 must be positive, got -1"
}

// ===========================================================================
// Section 5: Recursive Macros
// ===========================================================================

fn recursive_macros() {
    println!("\n=== Section 5: Recursive Macros ===\n");

    // --- Step 18: Recursive counting with tt ---
    // Each recursion peels off one tt, base case has zero tts
    macro_rules! count_tts {
        () => { 0usize };
        ($first:tt $($rest:tt)*) => { 1 + count_tts!($($rest)*) };
    }

    println!("  count_tts!(): {}", count_tts!());
    println!("  count_tts!(a b c): {}", count_tts!(a b c));
    println!("  count_tts!(+ - * /): {}", count_tts!(+ - * /));
    println!(
        "  count_tts!((group) single): {}",
        count_tts!((group) single)
    );

    // --- Step 19: Recursion pitfall ---
    // $val - 1 is token-pasted, not evaluated — leads to infinite expansion
    //
    // macro_rules! bad_countdown {
    //     (0) => { println!("done!") };
    //     ($n:expr) => {
    //         println!("{}", $n);
    //         bad_countdown!($n - 1);  // Becomes: bad_countdown!(((5) - 1) - 1)
    //     };                            // Tokens grow forever, never match `0`
    // }
    //
    // bad_countdown!(5);  // Would hit recursion limit!

    // Correct approach: peel off tokens instead of doing arithmetic
    macro_rules! count_down {
        () => {};
        ($head:tt $($rest:tt)*) => {
            println!("  count_down: {} remaining", count_tts!($head $($rest)*));
            count_down!($($rest)*);
        };
    }

    count_down!(a b c);

    // --- Step 20: tt processor ---
    // Process tokens one at a time with head/tail recursion
    macro_rules! tt_processor {
        () => {};
        (+ $($rest:tt)*) => {
            print!("[PLUS]");
            tt_processor!($($rest)*);
        };
        (- $($rest:tt)*) => {
            print!("[MINUS]");
            tt_processor!($($rest)*);
        };
        (* $($rest:tt)*) => {
            print!("[STAR]");
            tt_processor!($($rest)*);
        };
        ($head:tt $($rest:tt)*) => {
            print!("[{}]", stringify!($head));
            tt_processor!($($rest)*);
        };
    }

    print!("  tt_processor!(a + b - c * d): ");
    tt_processor!(a + b - c * d);
    println!();

    // --- Step 21: Internal rules with @ ---
    // Convention: @name arms are private helpers, not part of public API
    macro_rules! sorted_vec {
        ($($val:expr),* $(,)?) => {
            sorted_vec!(@build $($val),*)
        };
        (@build $($val:expr),*) => {{
            let mut v = vec![$($val),*];
            v.sort();
            v
        }};
    }

    let sv = sorted_vec!(3, 1, 4, 1, 5, 9);
    println!("  sorted_vec!(3,1,4,1,5,9): {:?}", sv);

    // --- Step 22: Macro calling macro ---
    // One macro can invoke another macro in its expansion
    macro_rules! log_level {
        (error, $msg:expr) => {
            println!("  [ERROR] {}", $msg)
        };
        (warn, $msg:expr) => {
            println!("  [WARN]  {}", $msg)
        };
        (info, $msg:expr) => {
            println!("  [INFO]  {}", $msg)
        };
    }

    // Wrapper macro delegates to log_level!
    macro_rules! log_error {
        ($msg:expr) => {
            log_level!(error, $msg)
        };
    }

    // Self-invocation for default argument
    macro_rules! log_msg {
        ($msg:expr) => {
            log_msg!(info, $msg)
        };
        ($level:ident, $msg:expr) => {
            log_level!($level, $msg)
        };
    }

    log_error!("disk full");
    log_msg!("server started");
    log_msg!(warn, "high memory usage");
}

// ===========================================================================
// Section 6: Export, Scoping, and Follow-Set Rules
// ===========================================================================

fn export_scoping_and_follow_set() {
    println!("\n=== Section 6: Export, Scoping, and Follow-Set Rules ===\n");

    // --- Step 23: $crate metavariable ---
    // $crate resolves to the crate where the macro is defined.
    // Essential for exported macros to reference their own items.
    //
    //   #[macro_export]
    //   macro_rules! create_thing {
    //       () => { $crate::Thing::new() };
    //   }
    //
    // Without $crate, the macro would look for Thing in the caller's crate.
    println!("  $crate resolves to the defining crate's root path.");
    println!("  Without it, exported macros break when called from other crates.");
    println!("  Example: $crate::MyType::new() instead of MyType::new()");

    // --- Step 24: #[macro_export] and Rust 2024 scoping ---
    println!("\n  #[macro_export] makes a macro available to other crates.");
    println!("  It places the macro at the crate root, regardless of definition location.");
    println!("  In Rust 2024: macros follow module scoping like other items.");
    println!("  No textual ordering requirement — call before or after definition.");
    println!("  Use `use crate::macro_name;` to bring exported macros into scope.");

    // --- Step 25: Follow-set restrictions ---
    // Only specific tokens can follow certain fragment specifiers.
    // This prevents ambiguous parsing as the language evolves.
    println!("\n  Follow-set restrictions (what can follow each specifier):");
    println!("    $e:expr      → => , ;");
    println!("    $s:stmt      → => , ;");
    println!("    $t:ty        → => , = | ; : > >> [ {{ as where");
    println!("    $p:pat       → => , = if in");
    println!("    $i:ident     → (anything)");
    println!("    $tt:tt       → (anything)");
    println!("    $l:literal   → (anything)");
    println!("    $lt:lifetime → (anything)");

    // Demonstrating => as a valid separator between expressions
    macro_rules! compose_alt {
        ($a:expr => $b:expr) => {{
            let first = $a;
            let second = $b;
            (first, second)
        }};
    }

    let pair = compose_alt!(1 + 1 => 2 + 2);
    println!("  compose_alt!(1+1 => 2+2): {:?}", pair);

    // Using ; as separator
    macro_rules! both {
        ($a:expr ; $b:expr) => {
            ($a, $b)
        };
    }

    let pair = both!(10 ; 20);
    println!("  both!(10 ; 20): {:?}", pair);
}

// ===========================================================================
// Section 7: Practical Declarative Macros
// ===========================================================================

fn practical_declarative() {
    println!("\n=== Section 7: Practical Declarative Macros ===\n");

    // --- Step 26: DSL with literal token keywords ---
    // Macros can match literal tokens to create natural-language-like syntax.
    // Note: follow-set rules restrict what can follow $:expr (only => , ;).
    // Use tt or literal for tokens followed by keywords.
    macro_rules! exchange {
        (Give $item:tt to $target:tt) => {
            println!("  → Giving {} to {}", $item, $target)
        };
        (Take $item:tt from $target:tt) => {
            println!("  ← Taking {} from {}", $item, $target)
        };
    }

    exchange!(Give "gold" to "merchant");
    exchange!(Take "potion" from "chest");
    exchange!(Give 100 to "bank");

    // --- Step 27: Function composition ---
    // Chain functions left-to-right using recursive expansion
    macro_rules! chain {
        ($val:expr => $f:expr) => { $f($val) };
        ($val:expr => $f:expr, $($rest:expr),+) => {
            chain!($f($val) => $($rest),+)
        };
    }

    let result = chain!(5 => |x| x + 1, |x| x * 2, |x: i32| x.to_string());
    println!("  chain!(5 => +1, *2, to_string): {result}");

    let result = chain!(vec![3, 1, 2] => |mut v: Vec<i32>| { v.sort(); v }, |v: Vec<i32>| v.len());
    println!("  chain!(sort then len): {result}");

    // --- Step 28: Enum dispatch ---
    // Generate match arms for all enum variants
    macro_rules! enum_display {
        ($enum_name:ident { $($variant:ident),+ $(,)? }) => {
            #[derive(Debug)]
            enum $enum_name { $($variant),+ }

            impl std::fmt::Display for $enum_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        $(Self::$variant => write!(f, "{}", stringify!($variant)),)+
                    }
                }
            }
        };
    }

    enum_display!(Direction {
        North,
        South,
        East,
        West
    });
    println!("  enum_display!: {}, {}", Direction::North, Direction::West);

    // Dispatch strings to handler code
    macro_rules! dispatch {
        ($val:expr, { $($variant:ident => $handler:expr),+ $(,)? }) => {
            match $val {
                $(stringify!($variant) => { $handler },)+
                other => println!("  dispatch: unknown command: {other}"),
            }
        };
    }

    for cmd in ["start", "stop", "explode"] {
        dispatch!(cmd, {
            start => println!("  dispatch: starting..."),
            stop => println!("  dispatch: stopping...")
        });
    }

    // --- Step 29: Limitations summary ---
    println!("\n  Declarative macro limitations:");
    println!("    - No type introspection: can't check if T implements a trait");
    println!("    - Purely syntactic: can't compute values during expansion");
    println!("    - No dynamic identifiers: can't build idents from strings");
    println!("    - Hygiene limits: items leak, variables don't");
    println!("    - Error messages can be cryptic for users");
    println!("  → When you hit these limits, reach for procedural macros");
}

// ===========================================================================
// Section 8: Proc Macros — Derive
// ===========================================================================

fn proc_macros_derive() {
    println!("\n=== Section 8: Proc Macros — Derive ===\n");

    // --- Step 30: Three kinds overview ---
    println!("  Procedural macros — three kinds:");
    println!("  {:<20} {:<28} {}", "Kind", "Invocation", "Behavior");
    println!("  {:<20} {:<28} {}", "────", "──────────", "────────");
    println!(
        "  {:<20} {:<28} {}",
        "Custom derive", "#[derive(Name)]", "Adds code alongside item"
    );
    println!(
        "  {:<20} {:<28} {}",
        "Attribute", "#[name(...)]", "Replaces the annotated item"
    );
    println!(
        "  {:<20} {:<28} {}",
        "Function-like", "name!(...)", "Arbitrary token transformation"
    );
    println!();

    // --- Step 31: Ecosystem: syn, quote, proc-macro2 ---
    println!("  Ecosystem crates for writing proc macros:");
    println!("    syn         — parses TokenStream into a typed AST (DeriveInput, ItemFn, ...)");
    println!("    quote       — quasi-quoting: write Rust code with #variable interpolation");
    println!("    proc-macro2 — wrapper around proc_macro types, usable outside macro crates");
    println!();
    println!("  Workspace pattern:");
    println!("    proc_macros/     (proc-macro crate — thin entry points)");
    println!("    proc_macros/src/ (implementation using proc_macro2 for testability)");
    println!();

    // --- Step 32: #[derive(Greet)] ---
    // Source: proc_macros/src/greet.rs
    // Flow: parse DeriveInput → extract ident → quote! { impl #name { fn greet() } }
    #[derive(Greet)]
    struct Robot;

    #[derive(Greet)]
    struct Sensor;

    println!("  {}", Robot.greet());
    println!("  {}", Sensor.greet());

    // The greet() method is generated — not hand-written.
    // DeriveInput gives us the struct name; quote! generates the impl block.

    // --- Step 33: #[derive(Builder)] ---
    // Source: proc_macros/src/builder.rs
    // Generates: FooBuilder struct with Option<T> fields, setter methods, build()
    #[derive(Builder)]
    struct ServerConfig {
        host: String,
        port: u16,
        max_connections: u32,
    }

    let cfg = ServerConfig::builder()
        .host("localhost".into())
        .port(8080)
        .max_connections(100)
        .build();
    println!(
        "  Builder: {}:{} (max {})",
        cfg.host, cfg.port, cfg.max_connections
    );

    // Missing fields panic at runtime with a clear message
    let result = std::panic::catch_unwind(|| ServerConfig::builder().host("x".into()).build());
    println!("  Builder missing field: panics = {}", result.is_err());

    // --- Step 34: Builder with #[rename] and #[builder_defaults] ---
    // #[rename("name")] changes the setter method name
    #[derive(Builder)]
    struct ApiClient {
        #[rename("url")]
        endpoint: String,
        #[rename("with_timeout")]
        timeout_ms: u64,
    }

    let client = ApiClient::builder()
        .url("https://api.example.com".into())
        .with_timeout(5000)
        .build();
    println!(
        "  Renamed setters: endpoint={}, timeout={}ms",
        client.endpoint, client.timeout_ms
    );

    // #[builder_defaults] uses Default::default() for unset fields
    #[derive(Builder)]
    #[builder_defaults]
    struct Settings {
        retries: u32,
        verbose: bool,
        buffer_size: usize,
    }

    let defaults = Settings::builder().build();
    println!(
        "  builder_defaults (all default): retries={}, verbose={}, buffer={}",
        defaults.retries, defaults.verbose, defaults.buffer_size
    );

    let custom = Settings::builder().retries(3).verbose(true).build();
    println!(
        "  builder_defaults (partial): retries={}, verbose={}, buffer={}",
        custom.retries, custom.verbose, custom.buffer_size
    );

    // With #[builder_defaults], the macro generates quote_spanned! assertions
    // that verify each field type implements Default. If not, the error
    // points at the field's type, not at #[derive(Builder)].
}

// ===========================================================================
// Section 9: Proc Macros — Attribute and Function-Like
// ===========================================================================

fn proc_macros_attribute_and_fn() {
    println!("\n=== Section 9: Proc Macros — Attribute and Function-Like ===\n");

    // --- Step 35: #[public] — make fields pub ---
    // Attribute macro: replaces the input, re-emitting with all fields pub.
    // Source: proc_macros/src/public.rs
    #[public]
    #[derive(Debug)]
    struct DatabaseConfig {
        host: String,
        port: u16,
        name: String,
    }

    // All fields are now pub — can construct directly
    let db = DatabaseConfig {
        host: "localhost".into(),
        port: 5432,
        name: "mydb".into(),
    };
    println!("  #[public]: {db:?}");

    // --- Step 36: #[public(exclude(secret))] ---
    // ExcludedFields Parse impl checks for exclude(...) in attr tokens.
    // Listed fields keep their original visibility.
    #[public(exclude(password))]
    #[derive(Debug)]
    struct UserAccount {
        username: String,
        email: String,
        password: String,
    }

    // username and email are pub; password keeps original visibility
    let user = UserAccount {
        username: "alice".into(),
        email: "alice@example.com".into(),
        password: "s3cret".into(),
    };
    println!(
        "  #[public(exclude)]: user={}, email={}",
        user.username, user.email
    );

    // The Parse impl: input.parse::<MetaList>() → check path.is_ident("exclude")
    // → Punctuated::<Ident, Token![,]>::parse_terminated on the inner tokens

    // --- Step 37: private! — generate getters ---
    // Function-like: clones input (re-emits struct) + generates impl with getters.
    // Uses format_ident!("get_{}", field_name) to construct method names.
    // Source: proc_macros/src/private.rs
    private!(
        struct Measurement {
            label: String,
            value: f64,
            unit: String,
        }
    );

    let m = Measurement {
        label: "temperature".into(),
        value: 36.6,
        unit: "°C".into(),
    };
    println!(
        "  private! getters: {}={}{}",
        m.get_label(),
        m.get_value(),
        m.get_unit()
    );

    // Unlike derive, function-like output replaces the input entirely.
    // That's why private.rs clones the input TokenStream and re-emits it.

    // --- Step 38: resource! DSL ---
    // Custom keywords via syn::custom_keyword! (resource, name, count).
    // Parse impl uses loop + peek + parse pattern:
    //   input.parse::<kw::resource>()?;
    //   while !input.is_empty() {
    //       if input.peek(kw::name) { parse name + LitStr }
    //       else if input.peek(kw::count) { parse count + LitInt }
    //   }
    // Source: proc_macros/src/resource.rs
    resource! {
        resource ConnectionPool
        name "postgres_pool"
        count 10
    }

    let pool = ConnectionPool::new();
    println!("  resource! DSL: {}", pool.description());
    assert_eq!(pool.name, "postgres_pool");
    assert_eq!(pool.count, 10);

    // Keywords can appear in any order
    resource! {
        resource CacheLayer
        count 256
        name "redis_cache"
    }

    let cache = CacheLayer::new();
    println!("  resource! (reordered): {}", cache.description());

    // --- Step 39: quote! repetition ---
    // In quote!, #(#var,)* iterates producing comma-separated items.
    println!("\n  quote! repetition patterns:");
    println!("    #(#item,)*      — comma-separated list");
    println!("    #(#item)*       — no separator (e.g., method definitions)");
    println!("    #(#k: #v,)*     — parallel iteration over two iterators");
    println!();
    println!("  Key rules:");
    println!("    - #var inside #(...)* must be an iterator");
    println!("    - No .property access in quote! — extract to let binding first");
    println!("    - No collect() needed — quote handles iteration directly");
    println!("    - Build conditional fragments outside quote!, then interpolate");
    println!();
    println!("  Example from builder.rs setter generation:");
    println!("    let setter_methods = fields.iter().map(|f| {{");
    println!("        let field_name = &f.ident;    // extract outside quote!");
    println!("        let ty = &f.ty;");
    println!("        quote! {{ pub fn #field_name(mut self, input: #ty) -> Self {{ ... }} }}");
    println!("    }});");
    println!("    quote! {{ #(#setter_methods)* }}  // expand all methods");
}

// ===========================================================================
// Section 10: Proc Macro Internals
// ===========================================================================

fn proc_macro_internals() {
    println!("\n=== Section 10: Proc Macro Internals ===\n");

    // --- Step 40: TokenStream and TokenTree ---
    println!("  TokenStream is a sequence of TokenTree values.");
    println!("  Four TokenTree variants:");
    println!("    Ident    — identifiers and keywords: foo, struct, i32");
    println!("    Punct    — single punctuation char: +, &, #, comma, semicolon");
    println!("    Literal  — literal values: 42, 3.14, \"hello\"");
    println!("    Group    — delimited tokens: (a, b), [1, 2], {{x + y}}");
    println!();
    println!("  Connection to declarative macros:");
    println!("    A `tt` fragment captures exactly one TokenTree.");
    println!("    A delimited group (a, b, c) is one tt — not three.");
    println!("    This is why count_tts!((a b c)) returns 1, not 3.");
    println!();
    println!("  proc_macro2 wraps proc_macro types so they can be used");
    println!("  outside proc macro crates (for testing, library code).");
    println!("  Use .into() to convert: proc_macro::TokenStream ↔ proc_macro2::TokenStream");
    println!();
    println!("  Typical entry point pattern:");
    println!("    #[proc_macro_derive(Name)]");
    println!("    pub fn derive_name(input: TokenStream) -> TokenStream {{");
    println!("        implementation(input.into()).into()  // convert both ways");
    println!("    }}");

    // --- Step 41: Parse trait and ParseStream ---
    println!("\n  The Parse trait (syn::parse::Parse) enables custom parsing:");
    println!("    impl Parse for MyType {{");
    println!("        fn parse(input: ParseStream) -> syn::Result<Self> {{ ... }}");
    println!("    }}");
    println!();
    println!("  ParseStream methods:");
    println!("    input.parse::<T>()?           — parse and consume a T");
    println!("    input.peek(Token![,])         — look ahead without consuming");
    println!("    input.peek(kw::name)          — peek for custom keyword");
    println!("    input.is_empty()              — check if all tokens consumed");
    println!("    Punctuated::parse_terminated  — parse comma-separated list");
    println!();
    println!("  Concrete example — resource.rs Parse impl:");
    println!("    input.parse::<kw::resource>()?;    // consume custom keyword");
    println!("    let name: Ident = input.parse()?;  // consume identifier");
    println!("    if input.peek(kw::name) {{          // look ahead for 'name'");
    println!("        input.parse::<kw::name>()?;    // consume it");
    println!("        let lit: LitStr = input.parse()?;");
    println!("    }}");

    // --- Step 42: quote_spanned! ---
    println!("\n  quote_spanned! attaches source spans to generated tokens.");
    println!("  Without it, errors from generated code point at #[derive(...)],");
    println!("  which is confusing. With it, errors point at the user's code.\n");
    println!("  Example from Builder's #[builder_defaults]:");
    println!("    quote_spanned! {{ ty.span() =>");
    println!("        struct #assertion_ident where #ty: core::default::Default;");
    println!("    }}");
    println!();
    println!("  If a field's type doesn't implement Default:");
    println!("    Without quote_spanned! → error at #[derive(Builder)] line");
    println!("    With quote_spanned!    → error at the field's type annotation");
    println!();
    println!("  The pattern: a zero-sized struct with a where clause that");
    println!("  only compiles if the constraint is satisfied.");
}

// ===========================================================================
// Section 11: Debugging Macros
// ===========================================================================

fn debugging_macros() {
    println!("\n=== Section 11: Debugging Macros ===\n");

    // --- Step 43: cargo expand ---
    println!("  cargo expand — shows fully expanded code after macro processing.");
    println!("  Install:  cargo install cargo-expand");
    println!("  Usage:    cargo expand --lib > expanded.rs");
    println!("            cargo expand --lib path::to::module\n");

    println!("  Common proc macro error messages and what they mean:");
    println!("    1. \"expected item after attributes\"");
    println!("       → attribute macro returned tokens that aren't a valid item");
    println!("    2. \"expected value, found struct `Foo`\"");
    println!("       → generated code uses a type where a value is expected");
    println!("    3. \"cannot find value `x` in this scope\"");
    println!("       → generated code references a nonexistent variable (hygiene)");
    println!("    4. \"expected identifier, found string literal\"");
    println!("       → passed a string where quote! expected an ident");
    println!("    5. \"this function takes N arguments but M were supplied\"");
    println!("       → generated function call has wrong argument count\n");

    println!("  Debug tip: write the generated code by hand first.");
    println!("  Hand-written code gives better IDE diagnostics.");
    println!("  Once it works, port it back to the macro.\n");

    // --- Step 44: compile_error!, env!, include_str! ---
    // compile_error! causes a custom compile error — useful for macro arms
    // that reject invalid input
    //
    //   macro_rules! platform_code {
    //       (windows) => { /* windows impl */ };
    //       (linux) => { /* linux impl */ };
    //       ($other:ident) => {
    //           compile_error!(concat!("unsupported: ", stringify!($other)));
    //       };
    //   }

    macro_rules! require_feature {
        (enabled) => {
            println!("  compile_error! demo: feature is enabled")
        };
        ($other:ident) => {
            compile_error!(concat!("unknown feature: ", stringify!($other)));
        };
    }

    require_feature!(enabled);

    // env! reads environment variables at compile time
    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    println!("  env!: {} v{}", pkg_name, pkg_version);

    // option_env! returns Option — doesn't fail if variable is missing
    let ci = option_env!("CI").unwrap_or("not set");
    println!("  option_env!(\"CI\"): {ci}");

    // include_str! / include_bytes! embed file contents at compile time
    println!("  include_str!(\"file.txt\")  — embeds as &'static str");
    println!("  include_bytes!(\"data.bin\") — embeds as &'static [u8]");

    // For proc macros, use syn::Error for structured error reporting
    println!("\n  Proc macro error reporting (syn::Error):");
    println!("    syn::Error::new(span, \"message\")         — error at span");
    println!("    syn::Error::new_spanned(tokens, \"msg\")   — error spanning tokens");
    println!("    error.to_compile_error()                  — convert to TokenStream");
    println!("    Combine multiple: errors.into_iter().map(|e| e.to_compile_error())");
}

// ===========================================================================
// Section 12: Decision Guide
// ===========================================================================

fn decision_guide() {
    println!("\n=== Section 12: Decision Guide ===\n");

    // --- Step 45: When to use what ---
    println!("  When to use what — decision table:\n");
    println!("  {:<22} {}", "Tool", "Use when...");
    println!("  {:<22} {}", "────", "──────────");
    println!(
        "  {:<22} {}",
        "Plain function", "Fixed args, runtime behavior, simple logic"
    );
    println!(
        "  {:<22} {}",
        "Generic function", "Same logic for multiple types (T: Trait)"
    );
    println!(
        "  {:<22} {}",
        "macro_rules!", "Variadic args, code generation, DSLs, boilerplate"
    );
    println!(
        "  {:<22} {}",
        "Derive macro", "Auto-implement traits for structs/enums"
    );
    println!(
        "  {:<22} {}",
        "Attribute macro", "Transform/augment items (add pub, inject code)"
    );
    println!(
        "  {:<22} {}",
        "Function-like macro", "Custom syntax, DSLs needing type introspection"
    );

    println!();
    println!("  Rules of thumb:");
    println!("    1. Start with a function — simplest, best error messages");
    println!("    2. Add generics if you need type polymorphism");
    println!("    3. Use macro_rules! for variadic patterns or repeated code");
    println!("    4. Use proc macros only when declarative macros fall short:");
    println!("       - Need to inspect types or field names (syn)");
    println!("       - Need to generate identifiers dynamically (format_ident!)");
    println!("       - Need to parse custom syntax (ParseStream)");
    println!("    5. Prefer derive over attribute when adding code (not replacing)");
    println!("    6. Use function-like proc macros for complex DSLs");
    println!();
    println!("  Complexity cost: function < generic < macro_rules! < proc macro");
    println!("  Always pick the simplest tool that solves the problem.");
}

// ===========================================================================
// Public entry point
// ===========================================================================

pub fn run() {
    macro_theory();
    declarative_basics();
    fragment_specifiers();
    arms_and_repetition();
    macro_hygiene();
    recursive_macros();
    export_scoping_and_follow_set();
    practical_declarative();
    proc_macros_derive();
    proc_macros_attribute_and_fn();
    proc_macro_internals();
    debugging_macros();
    decision_guide();
}
