use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::runtime::Builder;
use tokio::time;

mod mod_001_idioms_and_patterns;
mod mod_002_macros;
mod mod_003_threads_and_concurrency;
mod exercises;
mod mod_004_async_await_custom;
mod mod_004_async_await_and_tokio;
mod mod_006a_ffi_c;
mod mod_006b_ffi_python;

fn main() {

}

/*
## Constants

- Declared using the `const` keyword.
- Require an **explicit type annotation** — the type is never inferred.
- The value must be a **constant expression**, computable at compile
time.
- Constants have **no guaranteed fixed memory address**. The compiler
may inline them at each usage site or place them in read-only
memory — you cannot rely on a `const` having a single address.
This is the key difference from `static`, which is guaranteed to
have exactly one address.
- Constants cannot be `mut` — they are always immutable.
- Constants can be declared at any scope level: module-level or inside
a function body.
- A `const fn` is a function that can be evaluated at compile time.
Its return value can be used wherever a constant expression is
required (e.g., as the value of a `const` or `static`).
- **`const fn` limitations**: `const fn` cannot perform heap
allocation (no `String`, `Vec`, `Box::new`), call non-const
functions (no `.to_string()`, `.to_uppercase()`, etc.), or use
`dyn Trait`. Only a subset of operations is available at compile
time. However, `if`/`else`, `match`, `while`, and `loop` are all
supported in const fn.
- Naming convention: `SCREAMING_SNAKE_CASE`.
*/

// Module-level constant — visible throughout this module
const MAX_CONNECTIONS: u32 = 100;

fn constants() {
    // Function-local constant
    const TIMEOUT_SECONDS: u64 = 3600;

    println!("max connections: {MAX_CONNECTIONS}");
    println!("timeout: {TIMEOUT_SECONDS} seconds");

    // Constants can be used in constant expressions to define other constants
    const TIMEOUT_MILLIS: u64 = TIMEOUT_SECONDS * 1000;
    println!("timeout: {TIMEOUT_MILLIS} milliseconds");

    // const fn — a function evaluated at compile time, usable in
    // constant expressions
    const fn square(x: u32) -> u32 {
        x * x
    }
    const SQUARED: u32 = square(12);
    println!("const fn square(12) = {SQUARED}");

    // const fn with conditional logic — computed entirely at compile time
    const fn clamp(value: i32, min: i32, max: i32) -> i32 {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }
    const CLAMPED: i32 = clamp(150, 0, 100);
    println!("const fn clamp(150, 0, 100) = {CLAMPED}");

    // const fn with loops — fully supported in modern Rust
    const fn factorial(n: u64) -> u64 {
        let mut result = 1;
        let mut i = 2;
        while i <= n {
            result *= i;
            i += 1;
        }
        result
    }
    const FACT_10: u64 = factorial(10);
    println!("const fn factorial(10) = {FACT_10}");

    // const fn cannot use heap allocation, trait methods, or dyn:
    // const fn bad() -> String {
    //     String::from("hello") // ERROR: cannot call non-const fn `String::from`
    // }
    // const fn also_bad(s: &str) -> String {
    //     s.to_uppercase() // ERROR: cannot call non-const fn `to_uppercase`
    // }
}
