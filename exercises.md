## Idioms and Patterns (A1)

1. Unix Permission System (PhantomData + Boolean Const Generics)

   Build a Unix-style file permission system where read, write, and execute
   permissions are encoded as boolean const generics. A `FileHandle` is
   parameterized by its permission type so that only valid operations are
   available — e.g., you can only call `read()` on a handle with read
   permission. Invalid operations should be compile-time errors.

   * `Permission<const R: bool, const W: bool, const X: bool>` — zero-sized type with 3 boolean const generics (8 combinations)
   * `FileHandle<P>` parameterized by permission — conditional methods:
     - `Permission<true, _, _>` → `read()`
     - `Permission<_, true, _>` → `write()`
     - `Permission<_, _, true>` → `execute()`
   * `UserId(u32)` / `GroupId(u32)` newtypes (prevent argument swapping)
   * Extension trait `PermissionDescExt` on `&str`: parse `"rwx"`, `"r-x"`, `"rw-"` → `(bool,bool,bool)`
   * Marker trait `Auditable` with blanket impl only for readable file handles

2. Traffic Light Controller (Branching Type State)

   Model a traffic light as a type-state machine where each state is a
   distinct type. Transitions consume the old state and produce the new one,
   so the compiler enforces that only legal transitions are possible. Add an
   emergency mode that any state can enter, and use trait objects to manage
   an intersection where lights may be in different states simultaneously.

    * States: Red, Green, Yellow, FlashingRed — branching transition graph (not linear)
    * Red → Green, Green → Yellow, Yellow → Red, Any → FlashingRed (emergency), FlashingRed → Red
    * `Emergency` trait implemented generically for all `TrafficLight<S>` → consumes self
    * `LightStatus` trait with `status_line() -> Cow<str>` + `is_stop() -> bool`
    * Store `Vec<Box<dyn LightStatus>>` to manage intersection with lights in different states
    * Invalid transitions = compile-time errors

3. Color Space Library (Multiple Newtypes + Sealed Trait)

   Create a small color library with three representations — RGB, hex
   string, and HSL — each wrapped in its own newtype. Implement conversions
   between them using `From` and `TryFrom`, and unify them behind a sealed
   `ColorSpace` trait so that external crates cannot add new color types.

    * `Rgb((u8,u8,u8))` — always valid, `HexColor(String)` — validated via `TryFrom<&str>`, `Hsl((f64,f64,f64))` — validated ranges
    * `ColorError` enum: `InvalidHue`, `InvalidSaturation`, `InvalidLightness`, `InvalidHexFormat`
    * Triangle of conversions: `From<Rgb> for HexColor`, `From<&HexColor> for Rgb`, `From<Rgb> for Hsl`
    * Extension trait `ColorPaletteExt` on `[Rgb]`: `average_brightness()`, `most_saturated()`, `to_grayscale()`
    * Sealed trait `ColorSpace`: `to_rgb()`, `label()` — impl for all 3 types, used via `&dyn ColorSpace`

## Macros (A2)

4. Test Suite Runner Macro

   Write a macro that lets you define named test cases inline
   and generates a runner that executes each test, catches panics via
   `catch_unwind`, and reports pass/fail results — no `#[test]` harness
   needed. Then write a second macro that combines multiple suites into a
   single test report.

    * `test_suite!(MathTests { "name" => { body }, ... })` generates struct with `run()`, `summary()`, `all_passed()`
    * Each test block runs inside `std::panic::catch_unwind` — panics become FAIL, not crash
    * Internal rule `@run_single` wrapping each block
    * Second macro `test_group!(run_all: MathTests, StringTests)` combines suite results
    * Demonstrate with passing tests, assertion failures, and panics all caught safely

5. Bitflags Macro

   Build a macro that generates a complete bitflag type from a
   list of named constants and a backing integer type. The generated type
   should support combining flags with bitwise operators, querying
   individual flags, and pretty-printing the set of active flags. Prove
   that the macro works with different backing types by creating two
   separate flag sets.

    * `bitflags!(Permissions: u8 { READ = 0b0001, WRITE = 0b0010, ... })` generates complete flags type
    * Newtype with `Clone, Copy, PartialEq, Eq` + associated constants
    * Methods: `empty()`, `all()`, `contains()`, `insert()`, `remove()`, `toggle()`
    * Operator traits: `BitOr`, `BitAnd`, `Not`
    * `Display` prints set flags as `"READ | WRITE"` or `"(empty)"`
    * Second type `FileMode: u16` proves macro genericity across backing types

6. Builder Macro

   **Write a proc macro that takes a struct definition and
   automatically generates a builder type with chainable setter methods and
   a `build()` method that returns an error if any field was not set.
   This eliminates the boilerplate of hand-writing the builder pattern.

    * `make_builder!(Person { name: String, age: u32, email: String })` generates:
      - Builder struct with all fields as `Option<T>`
      - Chainable setter methods (take value, return `&mut Self`)
      - `build() -> Result<Person, String>` that checks all fields are set
    * Demonstrate builder pattern usage and missing-field errors**

## Threads/Concurrency + Async (A3, A4)

7. Parallel Word Frequency Counter

   Given a collection of text chunks, count word frequencies in parallel by
   spawning one thread per chunk. Each thread builds a local frequency map,
   then merges its results into a shared map protected by `Arc<Mutex<…>>`.
   Finally, print the most frequent words sorted by count.

    * Input: `Vec<String>` of text chunks
    * Spawn one thread per chunk, each counts words into local `HashMap<String, usize>`
    * Merge results into shared `Arc<Mutex<HashMap<String, usize>>>`
    * Print top most frequent words (sorted by count)
    * Uses: `thread::spawn`, `Arc`, `Mutex`, `JoinHandle`

8. Producer-Consumer Pipeline with Channels

   Build a multi-stage pipeline using channels: three producer threads
   generate numbers, a filter thread keeps only primes, and a collector
   thread gathers and prints the results. This exercises `mpsc` channels
   with multiple cloned senders and coordinated shutdown.

    * 3 producer threads each generate numbers (ranges 1..100, 100..200, 200..300)
    * Send through `mpsc::channel` to a single filter thread
    * Filter thread keeps only prime numbers, forwards via second channel to collector
    * Collector prints all primes sorted
    * Uses: `mpsc::channel`, multiple senders (clone), `thread::spawn`

9. Custom Countdown Future

   Implement the `Future` trait by hand to understand what async/await
   compiles down to. Your `CountdownFuture` decrements a counter on each
   `poll()`, returning `Pending` until it reaches zero. Write a minimal
   `block_on` executor to drive it, then combine two futures with a custom
   `JoinTwo` combinator.

    * Implement `CountdownFuture` struct that implements `Future<Output = String>`
    * Each `poll()` decrements an internal counter, prints progress, returns `Pending` until 0
    * Implement simple `block_on` executor using `Waker::noop()`
    * Combine two countdowns via a `JoinTwo` future combinator
    * Uses: `Future`, `Poll`, `Pin`, `Context`, `Waker`