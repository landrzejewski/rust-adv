use std::cell::{Cell, OnceCell, RefCell};
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::{Rc, Weak};

// ============================================================
// Section 1: Smart Pointers and Ownership Control
// ============================================================

/*
## Smart Pointers and Ownership Control

- **Smart pointers** are data structures that act like pointers but
  carry additional metadata and capabilities. Unlike plain references
  (`&T`), smart pointers **own** the data they point to.
- Smart pointers typically implement **`Deref`** (so they can be used
  like references) and **`Drop`** (so they clean up automatically when
  they go out of scope). `Box<T>`, `Rc<T>`, and `RefCell<T>` follow
  this pattern. `Cell<T>` is a related interior-mutability primitive
  but does NOT implement `Deref` — see its section below.

### `Box<T>` — Heap Allocation
- Allocates data on the **heap** with a fixed-size pointer on the stack.
- Use cases: recursive types (the compiler needs a known size),
  transferring ownership of large data without copying, and trait
  objects (`Box<dyn Trait>`).
- Zero runtime overhead compared to a raw heap pointer.

### `Rc<T>` — Reference Counting (Single-Threaded)
- Enables **multiple owners** of the same heap data. An internal
  counter tracks how many `Rc` pointers exist; the data is dropped
  when the count reaches zero.
- `Rc::clone(&rc)` increments the reference count — it does **not**
  deep-copy the data. This is an O(1) operation.
- `Rc<T>` provides **shared (immutable) access** only. For mutation,
  combine with `RefCell<T>` (see below).
- Single-threaded only. For multi-threaded shared ownership, use
  `Arc<T>` (covered in the concurrency module).

### `Weak<T>` — Non-Owning References
- A **weak reference** does not increment the strong count, so it
  does not prevent the data from being dropped.
- Created via `Rc::downgrade(&rc)`. Accessing the data requires
  `weak.upgrade()`, which returns `Option<Rc<T>>` — `None` if
  the data has already been dropped.
- Primary use: **breaking reference cycles** (e.g., parent ↔ child
  relationships) that would otherwise leak memory.

### `Cell<T>` — Interior Mutability (Primarily for Copy Types)
- Provides **interior mutability**: you can mutate the value inside
  even through a shared `&Cell<T>` reference.
- `get()` requires `T: Copy` — it returns a bitwise copy of the value.
  `set()` works with **any type** — it replaces the value, dropping
  the old one. For non-Copy types: `replace()` swaps and returns the
  old value, `take()` (requires `T: Default`) replaces with default
  and returns the old value, and `into_inner()` unwraps by consuming
  the Cell. Most commonly used with Copy types because `get()` is
  the primary access method.
- Zero runtime overhead — no borrow checking at runtime.
- Note: `Cell<T>` is **not** a smart pointer in the strict sense —
  it does NOT implement `Deref`. It is an interior mutability
  primitive. You access the value via `get()`/`set()`, not via
  dereferencing.

### `RefCell<T>` — Interior Mutability with Runtime Borrow Checking
- Like `Cell` but works with **any type** (not just `Copy`).
- Enforces Rust's borrowing rules **at runtime**: one mutable
  borrow OR any number of immutable borrows at a time.
- `borrow()` returns `Ref<T>` (shared), `borrow_mut()` returns
  `RefMut<T>` (exclusive). Violating the rules **panics** at
  runtime.
- `try_borrow()` and `try_borrow_mut()` return `Result` instead
  of panicking — use these when borrow conflicts are possible.

### `OnceCell<T>` — Single-Assignment Lazy Initialization
- A cell that can be written to **exactly once**. After
  initialization, it provides `&T` access without overhead.
- `get()` returns `Option<&T>`, `set(value)` returns
  `Result<(), T>` (error if already initialized),
  `get_or_init(|| ...)` initializes on first access.
- Single-threaded. The thread-safe counterpart `OnceLock` was
  covered earlier in this module (statics section).

### Related: `Pin<P>` — Pinning Smart Pointer
- `Pin<P>` is a wrapper that prevents the pointed-to value from being
  moved in memory. It is used with self-referential types and is
  critical for async/await. See advanced module 004 (section 2) for
  full coverage of `Pin`, `Unpin`, and `Box::pin`.
*/

// --- Box: recursive type ---
#[derive(Debug)]
#[allow(dead_code)]
enum List {
    Cons(i32, Box<List>),
    Nil,
}

// --- Box: trait object ---
trait Draw {
    fn draw(&self) -> String;
}

struct Circle {
    radius: f64,
}
impl Draw for Circle {
    fn draw(&self) -> String {
        format!("Circle(r={})", self.radius)
    }
}

struct Square {
    side: f64,
}
impl Draw for Square {
    fn draw(&self) -> String {
        format!("Square(s={})", self.side)
    }
}

// --- Rc + Weak: graph node with back-reference ---
#[derive(Debug)]
struct Node {
    value: i32,
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Rc<Node>>>,
}

fn smart_pointers_and_ownership_control() {
    // --- Box: recursive type ---
    // Without Box, `List` would have infinite size. Box provides
    // indirection: the Cons variant stores a fixed-size pointer.
    use List::{Cons, Nil};
    let list = Cons(1, Box::new(Cons(2, Box::new(Cons(3, Box::new(Nil))))));
    println!("recursive list: {list:?}");

    // --- Box: trait objects ---
    // Box<dyn Draw> stores any type implementing Draw on the heap.
    // The concrete type is erased; dispatch happens at runtime.
    let shapes: Vec<Box<dyn Draw>> = vec![
        Box::new(Circle { radius: 3.0 }),
        Box::new(Square { side: 5.0 }),
    ];
    for shape in &shapes {
        println!("  shape: {}", shape.draw());
    }

    // --- Rc: shared ownership ---
    let shared = Rc::new(String::from("shared data"));
    let owner2 = Rc::clone(&shared); // increments count, no deep copy
    let owner3 = Rc::clone(&shared);
    println!("Rc strong count: {}", Rc::strong_count(&shared)); // 3
    println!("owner2: {owner2}, owner3: {owner3}");
    drop(owner3);
    println!("after drop: strong count = {}", Rc::strong_count(&shared)); // 2

    // --- Weak: breaking reference cycles ---
    // Parent owns children (Rc), children hold a Weak back-reference
    // to the parent — no reference cycle, no memory leak.
    let parent = Rc::new(Node {
        value: 1,
        parent: RefCell::new(Weak::new()),
        children: RefCell::new(vec![]),
    });

    let child = Rc::new(Node {
        value: 2,
        parent: RefCell::new(Rc::downgrade(&parent)),
        children: RefCell::new(vec![]),
    });

    parent.children.borrow_mut().push(Rc::clone(&child));

    // upgrade() returns Some if the parent is still alive
    if let Some(p) = child.parent.borrow().upgrade() {
        println!("child's parent value: {}", p.value);
    }
    println!(
        "parent strong={}, weak={}",
        Rc::strong_count(&parent),
        Rc::weak_count(&parent)
    );

    // --- Cell: interior mutability for Copy types ---
    let counter = Cell::new(0_i32);
    counter.set(counter.get() + 1); // mutate through shared reference
    counter.set(counter.get() + 1);
    println!("Cell counter: {}", counter.get()); // 2

    // --- RefCell: interior mutability with runtime borrow checking ---
    // Caches an expensive computation result.
    let cache: RefCell<Option<String>> = RefCell::new(None);

    // First access: compute and cache
    {
        let mut val = cache.borrow_mut();
        if val.is_none() {
            *val = Some(String::from("computed result"));
        }
    } // mutable borrow ends here

    // Subsequent access: read from cache
    println!("cached: {}", cache.borrow().as_ref().unwrap());

    // try_borrow_mut avoids panics when a borrow conflict is possible
    let data = RefCell::new(42);
    let shared_borrow = data.borrow();
    match data.try_borrow_mut() {
        Ok(_) => println!("got mutable borrow"),
        Err(_) => println!("RefCell: mutable borrow failed (already borrowed)"),
    }
    drop(shared_borrow);

    // --- OnceCell: write-once lazy init ---
    let config: OnceCell<String> = OnceCell::new();
    assert!(config.get().is_none());

    // get_or_init runs the closure only on the first call
    let value = config.get_or_init(|| String::from("initialized"));
    println!("OnceCell: {value}");

    // Second call returns the same value, closure is not executed
    let same = config.get_or_init(|| panic!("this never runs"));
    println!("OnceCell again: {same}");

    // --- Rc<RefCell<T>>: shared mutable data ---
    let shared_list = Rc::new(RefCell::new(vec![1, 2, 3]));
    let handle_a = Rc::clone(&shared_list);
    let handle_b = Rc::clone(&shared_list);

    handle_a.borrow_mut().push(4);
    handle_b.borrow_mut().push(5);
    println!("shared mutable list: {:?}", shared_list.borrow());

    println!("smart_pointers_and_ownership_control section executed");
}

// ============================================================
// Section 2: Supertraits, Extension Traits, and Blanket Impls
// ============================================================

/*
## Supertraits, Extension Traits, and Blanket Implementations

### Supertraits
- A **supertrait** is a trait bound on another trait definition:
  `trait Loggable: Describable` means any type implementing
  `Loggable` must also implement `Describable`.
- The subtrait's methods (including defaults) can call methods
  from the supertrait, because the compiler guarantees they exist.
- Multiple supertraits: `trait B: A + C + D`.
- This is **not** OOP inheritance. There is no shared state, no
  method resolution order, no "is-a" relationship. It is purely a
  **constraint**: implementing the subtrait requires implementing
  the supertrait(s).
- A function with a subtrait bound can call methods from both the
  subtrait and all its supertraits.
- Standard library example: `std::error::Error: Display + Debug`.

### Extension Traits
- **Extension traits** add methods to types you do not own (foreign
  types) without wrapping them in a newtype.
- Recipe: define a trait with the desired methods, implement it for
  the foreign type. Callers must `use` the extension trait.
- Naming convention: suffix with `Ext` (e.g., `StrExt`, `IterExt`).
- Limitation: the extension trait must be in scope (`use`d) for its
  methods to be callable. This is by design — it prevents global
  namespace pollution.

### Blanket Implementations
- A **blanket implementation** implements a trait for all types
  satisfying some bound: `impl<T: Display> MyTrait for T { ... }`.
- Standard library examples:
  - `impl<T: Display> ToString for T` — every `Display` type gets
    `.to_string()` automatically.
  - `impl<T> From<T> for T` — every type can convert from itself.
- Blanket impls can provide **method bodies** that build on the
  bounded trait's methods. This gives every qualifying type new
  functionality without per-type implementations.
- **Overlap restriction**: two blanket impls must not apply to the
  same type. The compiler rejects conflicting implementations.
- **Associated types** in blanket impls can adapt to the concrete
  type's properties, enabling flexible generic behavior.
*/

// --- Supertraits ---
trait Describable {
    fn describe(&self) -> String;
}

// Loggable requires Describable as a supertrait.
// Any type implementing Loggable must also implement Describable.
trait Loggable: Describable {
    fn log_level(&self) -> &str;

    // Default method — can call describe() from the supertrait
    fn log(&self) {
        println!("[{}] {}", self.log_level(), self.describe());
    }
}

#[allow(dead_code)]
struct ServerEvent {
    message: String,
    severity: u8,
}

impl Describable for ServerEvent {
    fn describe(&self) -> String {
        format!("event: {}", self.message)
    }
}

impl Loggable for ServerEvent {
    fn log_level(&self) -> &str {
        match self.severity {
            0..=3 => "INFO",
            4..=7 => "WARN",
            _ => "ERROR",
        }
    }
}

// --- Extension trait ---
trait StrExt {
    fn is_blank(&self) -> bool;
    fn word_count(&self) -> usize;
}

impl StrExt for str {
    fn is_blank(&self) -> bool {
        self.trim().is_empty()
    }

    fn word_count(&self) -> usize {
        self.split_whitespace().count()
    }
}

// --- Blanket impl providing behavior ---
// Every type implementing Display automatically gets summary().
trait Summarizable {
    fn summary(&self) -> String;
}

impl<T: fmt::Display> Summarizable for T {
    fn summary(&self) -> String {
        let full = self.to_string();
        if full.len() > 20 {
            // Byte-level slicing: safe for ASCII; for production code,
            // use char-aware truncation to avoid panics on multi-byte chars.
            format!("{}...", &full[..20])
        } else {
            full
        }
    }
}

// --- Trait with associated type (per-type impls via macro) ---
trait Invertible {
    type Output;
    fn invert(&self) -> Self::Output;
}

impl Invertible for bool {
    type Output = bool;
    fn invert(&self) -> bool {
        !self
    }
}

// Macro avoids repeating the impl for each signed integer type
macro_rules! impl_invertible_signed {
    ($($t:ty),*) => {
        $(
            impl Invertible for $t {
                type Output = $t;
                fn invert(&self) -> $t {
                    -self
                }
            }
        )*
    };
}
impl_invertible_signed!(i8, i16, i32, i64);

fn supertraits_extension_traits_and_blanket_impls() {
    // --- Supertraits ---
    let event = ServerEvent {
        message: String::from("server started"),
        severity: 1,
    };
    // Call supertrait method directly
    println!("describe: {}", event.describe());
    // Call subtrait default method (internally calls describe())
    event.log();

    // A function requiring the subtrait can call both sets of methods
    fn log_item(item: &impl Loggable) {
        item.log();
        println!("  description: {}", item.describe());
    }
    log_item(&event);

    // --- Extension trait ---
    // StrExt methods are available because the trait is defined above
    println!("\"  \".is_blank() = {}", "  ".is_blank());
    println!("\"hi\".is_blank() = {}", "hi".is_blank());
    println!(
        "\"hello world\".word_count() = {}",
        "hello world".word_count()
    );

    // --- Blanket impl: Summarizable for all Display types ---
    // i32 implements Display, so it gets summary() automatically
    println!("42.summary() = {}", 42_i32.summary());
    println!("\"short\".summary() = {}", "short".summary());
    // Long string gets truncated by summary()
    let long = "This is a very long string that exceeds twenty characters";
    println!("long.summary() = {}", long.summary());

    // --- Trait with associated type (per-type impls via macro) ---
    println!("true.invert() = {}", true.invert());
    println!("(-5_i32).invert() = {}", (-5_i32).invert());
    println!("7_i64.invert() = {}", 7_i64.invert());

    println!("supertraits_extension_traits_and_blanket_impls section executed");
}

// ============================================================
// Section 3: Marker Traits and Wrapper Structs
// ============================================================

/*
## Marker Traits and Wrapper Structs

### Marker Traits
- **Marker traits** are traits with **no methods**. They signal a
  capability or property to the compiler or downstream code.
- Standard library marker traits:
  - `Send` — the type can be transferred across thread boundaries.
  - `Sync` — the type can be shared between threads via `&T`.
  - `Sized` — the type has a known size at compile time (implicit
    bound on all type parameters; relax with `?Sized`).
  - `Copy` — the type can be implicitly copied (bitwise) on move.
- You can define **custom marker traits** to group requirements:
  `trait Storable: Clone + Debug + PartialEq {}`. Any type meeting
  all supertraits can then implement `Storable`.
- A **blanket implementation** makes a marker trait automatic:
  `impl<T: Clone + Debug + PartialEq> Storable for T {}` — now
  every qualifying type is `Storable` without explicit impls.
- **Pure tag markers** (no supertraits, no methods) act as opt-in
  capability flags. Only types with an explicit `impl` pass
  a bound requiring the tag.

### Wrapper Structs (Newtype Pattern)
- A **newtype** is a single-field tuple struct wrapping another
  type: `struct UserId(u32)`.
- Purposes:
  - **Type safety**: prevents accidentally passing a `ProductId`
    where a `UserId` is expected, even though both wrap `u32`.
  - **Orphan rule workaround**: implement a foreign trait on a
    foreign type by wrapping it in a local newtype.
  - **Encapsulation**: expose a limited API over the inner type.
- **Idiomatic construction**: implement `TryFrom<&str>` (or `From`)
  for validated newtypes instead of a bare `new()` method — this
  integrates with the `?` operator and standard conversion traits.
- **`Deref` warning**: implementing `Deref<Target = InnerType>` gives
  transparent access to the inner type's methods, but use with care.
  It blurs the type boundary — any method on the inner type becomes
  callable on the newtype, which can break abstraction (e.g., users
  can call `.push()` on a newtype wrapping `Vec` even if you intended
  to restrict the API). Prefer explicit `AsRef` or named accessor
  methods when the newtype should NOT expose the full inner API.
- Derive common traits (`Debug`, `Clone`, `PartialEq`, etc.) to
  make the newtype ergonomic.

### Sealed Traits
- A **sealed trait** is a public trait that **cannot be implemented
  outside the defining crate**. External code can use the trait as
  a bound and call its methods, but cannot add new implementations.
- The pattern:
  1. Define a private module: `mod sealed { pub trait Seal {} }`
  2. Make the public trait require the seal:
     `pub trait MyTrait: sealed::Seal { ... }`
  3. Implement `Seal` only for the types you want.
  External crates cannot access `sealed::Seal` (private module),
  so they cannot implement `MyTrait`.
- **Why seal a trait?**
  - Safely add methods with default implementations to the trait
    without breaking downstream code (no external impls exist).
  - Guarantee an exhaustive set of implementors (useful for
    match-like dispatch).
  - Standard library example: `std::slice::SliceIndex` is sealed.
- Note: `pub trait Seal` inside a private `mod` is `pub` within
  the module, but the module itself is inaccessible from outside.
*/

// --- Custom marker trait with blanket impl ---
trait Persistable: Clone + fmt::Debug {}

// Blanket impl: every Clone + Debug type is automatically Persistable
impl<T: Clone + fmt::Debug> Persistable for T {}

fn persist<T: Persistable>(item: &T) {
    let backup = item.clone();
    println!("persisting: {:?} (backup: {:?})", item, backup);
}

// --- Pure tag marker ---
trait Auditable {}

#[derive(Debug)]
#[allow(dead_code)]
struct Invoice {
    id: u32,
    amount: f64,
}
impl Auditable for Invoice {}

fn audit<T: Auditable + fmt::Debug>(item: &T) {
    println!("audit record: {:?}", item);
}

// --- Newtype with Deref ---
#[derive(Debug, Clone, PartialEq)]
struct EmailAddress(String);

impl EmailAddress {
    fn new(email: &str) -> Result<Self, String> {
        if email.contains('@') && email.contains('.') {
            Ok(Self(email.to_string()))
        } else {
            Err(format!("invalid email: {email}"))
        }
    }
}

// Idiomatic construction via TryFrom — integrates with the ? operator
impl TryFrom<&str> for EmailAddress {
    type Error = String;
    fn try_from(email: &str) -> Result<Self, Self::Error> {
        Self::new(email)
    }
}

// Deref lets EmailAddress be used wherever &str is expected.
// WARNING: this exposes ALL of str's methods (e.g., .len(), .contains()).
// If you need a more restricted API, use AsRef<str> or a named method instead.
impl Deref for EmailAddress {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- Sealed trait ---
// The sealed module is private — external crates cannot access Seal.
// The `pub trait Seal` is public *within* the module, which lets the
// outer trait reference it, but the module itself is not re-exported.
mod sealed {
    pub trait Seal {}
}

// DatabaseBackend is visible but requires Seal, which is inaccessible
// from outside this crate. Only types here can implement it.
trait DatabaseBackend: sealed::Seal {
    fn name(&self) -> &str;
    fn max_connections(&self) -> u32;
}

struct Postgres;
struct Sqlite;

impl sealed::Seal for Postgres {}
impl sealed::Seal for Sqlite {}

impl DatabaseBackend for Postgres {
    fn name(&self) -> &str {
        "PostgreSQL"
    }
    fn max_connections(&self) -> u32 {
        100
    }
}

impl DatabaseBackend for Sqlite {
    fn name(&self) -> &str {
        "SQLite"
    }
    fn max_connections(&self) -> u32 {
        1
    }
}

fn describe_backend(backend: &dyn DatabaseBackend) {
    println!(
        "{}: max {} connections",
        backend.name(),
        backend.max_connections()
    );
}

fn marker_traits_and_wrapper_structs() {
    // --- Marker trait with blanket impl ---
    // persist() works for any Clone + Debug type — the blanket impl
    // makes them all Persistable automatically.
    persist(&42);
    persist(&String::from("data"));
    persist(&vec![1, 2, 3]);

    // --- Pure tag marker ---
    // Only types with an explicit `impl Auditable` pass the bound.
    let inv = Invoice {
        id: 1,
        amount: 99.95,
    };
    audit(&inv);
    // audit(&42); // ERROR: `i32` does not implement `Auditable`

    // --- Newtype: EmailAddress ---
    let email = EmailAddress::new("user@example.com").unwrap();
    println!("email: {email}");

    // Deref coercion: EmailAddress acts like &str
    println!("email length: {}", email.len());
    println!("email contains 'example': {}", email.contains("example"));

    // TryFrom enables the ? operator and standard conversions
    let email2: Result<EmailAddress, _> = "admin@site.org".try_into();
    println!("email via TryFrom: {email2:?}");

    // Validation rejects invalid input
    let bad = EmailAddress::new("not-an-email");
    println!("invalid email result: {bad:?}");

    // --- Sealed trait ---
    describe_backend(&Postgres);
    describe_backend(&Sqlite);
    // External crates cannot implement DatabaseBackend:
    // struct MySql;
    // impl DatabaseBackend for MySql { ... }
    // ERROR: the trait bound `MySql: sealed::Seal` is not satisfied

    println!("marker_traits_and_wrapper_structs section executed");
}

// ============================================================
// Section 4: Marker Structs and Phantom Types
// ============================================================

/*
## Marker Structs and Phantom Types

- `std::marker::PhantomData<T>` is a **zero-sized type** (ZST) that
  tells the compiler "this struct is logically associated with `T`,
  even though it does not store one". It occupies **no memory** at
  runtime.
- **Why it exists**: Rust's drop checker and variance rules depend
  on what types a struct contains. If a struct is generic over `T`
  but never stores a `T`, the compiler lacks information about how
  `T` relates to the struct's lifetime or ownership.
  `PhantomData<T>` provides that missing information.
- **Common use cases**:
  - **Type-safe wrappers**: distinguishing otherwise identical
    structs at the type level (e.g., `Distance<Meters>` vs
    `Distance<Kilometers>`).
  - **Lifetime markers**: when a struct holds a raw pointer to `T`,
    `PhantomData<&'a T>` tells the compiler the struct borrows `T`
    for lifetime `'a`.
  - **Type state pattern** (section 5): phantom type parameters
    encode states without runtime overhead.
- `PhantomData` is constructed as `PhantomData` (unit-like syntax).
  There is no data to pass.
- Phantom type parameters need **no trait bounds** on the struct
  definition. Bounds belong on the `impl` blocks that use `T`.
- `PhantomData<T>` makes the struct act **as if it owns a `T`**
  (affects drop checking). When ownership semantics are not desired,
  use `PhantomData<*const T>` (covariant, no ownership) instead.
*/

// Marker types — zero-sized, never instantiated directly.
// Clone + Copy are required so that Distance<Unit> can derive Copy.
#[derive(Clone, Copy)]
struct Meters;
#[derive(Clone, Copy)]
struct Kilometers;

// Distance tagged with its unit at the type level.
// PhantomData<Unit> makes the Unit parameter "used" without storing
// any data — the struct is the same size as f64.
#[derive(Debug, Clone, Copy)]
struct Distance<Unit> {
    value: f64,
    _unit: PhantomData<Unit>,
}

impl<Unit> Distance<Unit> {
    fn new(value: f64) -> Self {
        Self {
            value,
            _unit: PhantomData,
        }
    }
}

// Conversion methods are only available on the correct unit type.
impl Distance<Kilometers> {
    fn to_meters(self) -> Distance<Meters> {
        Distance::new(self.value * 1000.0)
    }
}

impl Distance<Meters> {
    fn to_kilometers(self) -> Distance<Kilometers> {
        Distance::new(self.value / 1000.0)
    }
}

fn marker_structs_and_phantom_types() {
    let marathon = Distance::<Kilometers>::new(42.195);
    let in_meters = marathon.to_meters();
    println!(
        "marathon: {:.3} km = {:.1} m",
        marathon.value, in_meters.value
    );

    let hundred_m = Distance::<Meters>::new(100.0);
    let in_km = hundred_m.to_kilometers();
    println!("sprint: {} m = {} km", hundred_m.value, in_km.value);

    // Compile-time unit safety — mixing units is a type error:
    // fn needs_km(_d: Distance<Kilometers>) {}
    // needs_km(hundred_m); // ERROR: expected `Distance<Kilometers>`,
    //                      //        found `Distance<Meters>`

    // PhantomData is truly zero-sized:
    println!("size of f64: {} bytes", std::mem::size_of::<f64>());
    println!(
        "size of Distance<Meters>: {} bytes",
        std::mem::size_of::<Distance<Meters>>()
    );
    println!(
        "size of Distance<Kilometers>: {} bytes",
        std::mem::size_of::<Distance<Kilometers>>()
    );
    // All three print 8 bytes — PhantomData adds zero overhead.

    // --- PhantomData as a lifetime marker ---
    // When a struct holds a raw pointer, PhantomData<&'a T> tells the
    // compiler that the struct borrows T for lifetime 'a, preventing
    // use-after-free at compile time.
    struct RawSlice<'a, T> {
        ptr: *const T,
        len: usize,
        _marker: PhantomData<&'a T>, // "this borrows &'a T"
    }

    impl<'a, T: fmt::Debug> RawSlice<'a, T> {
        fn from_slice(slice: &'a [T]) -> Self {
            Self {
                ptr: slice.as_ptr(),
                len: slice.len(),
                _marker: PhantomData,
            }
        }

        fn get(&self, index: usize) -> Option<&'a T> {
            if index < self.len {
                // SAFETY: index is within bounds, and the lifetime 'a
                // guarantees the data is still alive.
                Some(unsafe { &*self.ptr.add(index) })
            } else {
                None
            }
        }
    }

    let data = vec![10, 20, 30];
    let raw = RawSlice::from_slice(&data);
    println!("raw slice [1] = {:?}", raw.get(1)); // Some(20)
    println!("raw slice [5] = {:?}", raw.get(5)); // None
    // Without PhantomData<&'a T>, the compiler would not know that
    // `raw` borrows `data`, and this code could silently dangle:
    // drop(data);
    // println!("{:?}", raw.get(0)); // would be use-after-free!

    println!("marker_structs_and_phantom_types section executed");
}

// ============================================================
// Section 5: Struct Tagging / Type State Pattern
// ============================================================

/*
## Struct Tagging / Type State Pattern

- The **type state pattern** uses phantom type parameters to
  encode an object's **state** in its type. The compiler then
  enforces valid state transitions — invalid transitions become
  compile-time errors, not runtime errors.
- The recipe:
  1. Define **marker structs** for each state (empty structs —
     zero-sized types, never instantiated directly).
  2. Parameterize the main struct over a state type:
     `struct Builder<State>` with a `PhantomData<State>` field.
  3. Implement methods **only for specific states**:
     `impl Builder<Incomplete>` has `set_url()` which returns
     `Builder<Complete>`.
  4. Transition methods **consume `self`** (move semantics) and
     return the struct with the new state type. This prevents
     using the old state after a transition.
- This is **zero-cost** at runtime. Phantom types and marker
  structs are ZSTs; the compiled code is identical to a non-generic
  version.
- **Comparison with the runtime builder pattern** (module 007):
  that builder uses `Option<T>` fields and checks at runtime
  whether required fields are set. The type state pattern moves
  these checks to compile time — if it compiles, the required
  fields are guaranteed to be set.
- **Limitation**: the number of type states should be small. For
  many states or dynamic state machines, an enum-based approach
  (runtime checking) is more practical.
*/

// State markers — zero-sized types
struct NoUrl;
struct HasUrl;

// Request builder parameterized by whether a URL has been set.
// This ensures send() is only callable after url() has been called.
#[allow(dead_code)]
struct RequestBuilder<UrlState> {
    url: Option<String>,
    timeout_ms: u32,
    _state: PhantomData<UrlState>,
}

// Methods available in ANY state (generic over S)
impl<S> RequestBuilder<S> {
    fn timeout(mut self, ms: u32) -> Self {
        self.timeout_ms = ms;
        self
    }
}

// Constructor — starts in the NoUrl state
impl RequestBuilder<NoUrl> {
    fn new() -> Self {
        Self {
            url: None,
            timeout_ms: 5000,
            _state: PhantomData,
        }
    }

    // Setting the URL transitions the state: NoUrl → HasUrl.
    // self is consumed — the NoUrl builder cannot be used afterward.
    fn url(self, url: &str) -> RequestBuilder<HasUrl> {
        RequestBuilder {
            url: Some(url.to_string()),
            timeout_ms: self.timeout_ms,
            _state: PhantomData,
        }
    }
}

// send() is ONLY available when the URL has been set (HasUrl state)
impl RequestBuilder<HasUrl> {
    fn send(&self) -> String {
        format!(
            "sending request to {} (timeout: {}ms)",
            self.url.as_ref().unwrap(),
            self.timeout_ms
        )
    }
}

fn struct_tagging_type_state_pattern() {
    // Build a request — url() must be called before send()
    let response = RequestBuilder::new()
        .timeout(3000)
        .url("https://example.com")
        .send();
    println!("{response}");

    // Order of url() and timeout() is flexible
    let response2 = RequestBuilder::new()
        .url("https://api.example.com/data")
        .timeout(1000)
        .send();
    println!("{response2}");

    // Compile-time enforcement — these would be type errors:
    //
    // Calling send() without setting a URL:
    //   RequestBuilder::new().send();
    //   ERROR: no method named `send` found for `RequestBuilder<NoUrl>`
    //
    // Reusing the builder after url() consumed it:
    //   let builder = RequestBuilder::new();
    //   let with_url = builder.url("https://x.com");
    //   builder.timeout(1);
    //   ERROR: use of moved value `builder`

    println!("struct_tagging_type_state_pattern section executed");
}

// ============================================================
// Section 6: Const Generics and Compile-Time Parameterization
// ============================================================

/*
## Const Generics and Compile-Time Parameterization

- **Const generics** allow types and functions to be parameterized
  over **constant values** (not just types):
  `struct Buffer<T, const N: usize>`.
- The const parameter must be a primitive type (`usize`, `bool`,
  `char`, integer types).
- `[T; N]` in the standard library uses const generics internally.
- **Multiple const parameters**: a type can have more than one
  const generic, e.g., `Matrix<const ROWS: usize, const COLS: usize>`.
  This lets the compiler enforce dimensional compatibility —
  a 2x3 matrix is a different type than a 3x2 matrix.
- **Const parameters in return types**: a function or method can
  return a type whose const parameters differ from the input's.
  For example, `transpose()` on a `Matrix<ROWS, COLS>` returns
  `Matrix<COLS, ROWS>`.
- **Compile-time capacity enforcement**: const generics make the
  size part of the type. `FixedStack<i32, 4>` is a different type
  than `FixedStack<i32, 8>` — you cannot accidentally mix them.
- **Limitations**: complex const expressions (like `{N + M}` in a
  type position) require the nightly-only `generic_const_exprs`
  feature. Stable Rust supports const parameters in simpler
  contexts and workarounds using helper const functions.
*/

// --- Matrix with two const parameters ---
#[derive(Clone)]
struct Matrix<const ROWS: usize, const COLS: usize> {
    data: [[f64; COLS]; ROWS],
}

impl<const ROWS: usize, const COLS: usize> Matrix<ROWS, COLS> {
    fn zero() -> Self {
        Self {
            data: [[0.0; COLS]; ROWS],
        }
    }

    fn get(&self, row: usize, col: usize) -> f64 {
        self.data[row][col]
    }

    fn set(&mut self, row: usize, col: usize, value: f64) {
        self.data[row][col] = value;
    }

    fn rows(&self) -> usize {
        ROWS
    }

    fn cols(&self) -> usize {
        COLS
    }

    // Transpose: ROWS x COLS → COLS x ROWS.
    // The return type's const parameters are swapped — the compiler
    // verifies dimensional consistency at compile time.
    fn transpose(&self) -> Matrix<COLS, ROWS> {
        let mut result = Matrix::<COLS, ROWS>::zero();
        for r in 0..ROWS {
            for c in 0..COLS {
                result.data[c][r] = self.data[r][c];
            }
        }
        result
    }
}

impl<const ROWS: usize, const COLS: usize> fmt::Debug for Matrix<ROWS, COLS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Matrix {}x{}:", ROWS, COLS)?;
        for row in &self.data {
            writeln!(f, "  {:?}", row)?;
        }
        Ok(())
    }
}

// --- Fixed-capacity stack ---
// CAP is part of the type: FixedStack<i32, 4> != FixedStack<i32, 8>.
// T: Copy is required because [None; CAP] copies the None value.
struct FixedStack<T, const CAP: usize> {
    data: [Option<T>; CAP],
    len: usize,
}

impl<T: Copy, const CAP: usize> FixedStack<T, CAP> {
    fn new() -> Self {
        Self {
            data: [None; CAP],
            len: 0,
        }
    }

    fn push(&mut self, value: T) -> Result<(), &'static str> {
        if self.len >= CAP {
            return Err("stack overflow: capacity exceeded");
        }
        self.data[self.len] = Some(value);
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        self.data[self.len].take()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn capacity(&self) -> usize {
        CAP
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// Const generic in a standalone function — works with any array size
fn sum_array<const N: usize>(arr: &[i32; N]) -> i32 {
    arr.iter().sum()
}

fn const_generics_and_compile_time_parameterization() {
    // --- Matrix ---
    let mut m = Matrix::<2, 3>::zero();
    m.set(0, 0, 1.0);
    m.set(0, 1, 2.0);
    m.set(0, 2, 3.0);
    m.set(1, 0, 4.0);
    m.set(1, 1, 5.0);
    m.set(1, 2, 6.0);
    println!("original ({}x{}):\n{:?}", m.rows(), m.cols(), m);

    // Transpose swaps dimensions: 2x3 → 3x2
    let t = m.transpose();
    println!("transposed ({}x{}):\n{:?}", t.rows(), t.cols(), t);
    println!("m[0][2] = {}, t[2][0] = {}", m.get(0, 2), t.get(2, 0));

    // Matrix<2,3> and Matrix<3,2> are different types:
    // let _: Matrix<2, 3> = t; // ERROR: expected Matrix<2, 3>,
    //                           //        found Matrix<3, 2>

    // --- FixedStack ---
    let mut stack = FixedStack::<i32, 4>::new();
    stack.push(10).unwrap();
    stack.push(20).unwrap();
    stack.push(30).unwrap();
    stack.push(40).unwrap();
    println!("stack len: {}, capacity: {}", stack.len(), stack.capacity());

    // Pushing past capacity returns an error
    let overflow = stack.push(50);
    println!("push overflow: {overflow:?}");

    // Pop returns items in LIFO order
    while !stack.is_empty() {
        print!("{} ", stack.pop().unwrap());
    }
    println!("(popped all)");

    // --- Const generic function ---
    let small = [1, 2, 3];
    let large = [10, 20, 30, 40, 50];
    println!("sum of {:?} = {}", small, sum_array(&small));
    println!("sum of {:?} = {}", large, sum_array(&large));

    println!("const_generics_and_compile_time_parameterization section executed");
}

// ============================================================
// Section 7: Cow<T> — Clone on Write
// ============================================================

/*
## Cow<T> — Clone on Write

`std::borrow::Cow<'a, B>` (Clone on Write) is an enum that holds
either a **borrowed reference** (`&'a B`) or an **owned value**
(`B::Owned`). It avoids unnecessary allocations when data may or
may not need to be modified.

```text
enum Cow<'a, B: ToOwned + ?Sized + 'a> {
    Borrowed(&'a B),       // no allocation
    Owned(<B as ToOwned>::Owned),  // owned, heap-allocated
}
```

### Why Cow Matters
- Functions that *sometimes* need to modify a string (or slice)
  can return `Cow<str>` instead of always allocating a `String`.
  If no modification is needed, the borrowed variant avoids a clone.
- Common in APIs that process text: sanitization, escaping,
  normalization — where most inputs pass through unchanged.

### Key Methods and Traits
- `Cow::Borrowed(s)` / `Cow::Owned(s)` — construct directly.
- `Cow<str>` implements `Deref<Target = str>`, so it can be used
  anywhere `&str` is expected.
- `.to_mut()` — returns `&mut B::Owned`, cloning on first call if
  currently `Borrowed`. Subsequent calls reuse the owned data.
- `.into_owned()` — consumes the `Cow`, returning the owned value
  (cloning if necessary).
- **From conversions**: `&str` → `Cow::Borrowed`, `String` →
  `Cow::Owned`. Same for `&[T]` / `Vec<T>`.

### When to Use Cow
- **Return type for functions that conditionally modify data** —
  avoids allocating when the input passes through unchanged.
- **Struct fields that may or may not own data** — e.g., error
  messages that are sometimes static strings, sometimes formatted.
- **Performance-sensitive text processing** — regex replace,
  escaping special characters, normalizing strings.
*/

use std::borrow::Cow;

// This function only allocates a new String when the input actually
// contains characters that need escaping. For clean inputs, it
// returns a zero-cost borrowed reference.
fn escape_html(input: &str) -> Cow<'_, str> {
    if input.contains(['<', '>', '&', '"']) {
        // Needs modification — allocate and return owned
        let escaped = input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        Cow::Owned(escaped)
    } else {
        // No special chars — return borrowed, zero allocation
        Cow::Borrowed(input)
    }
}

fn cow_clone_on_write() {
    // --- Basic usage: conditional allocation ---
    let clean = "Hello, world!";
    let dirty = "<script>alert('xss')</script>";

    let result1 = escape_html(clean);
    let result2 = escape_html(dirty);

    println!(
        "clean input:  {:?} (borrowed: {})",
        result1,
        matches!(result1, Cow::Borrowed(_))
    );
    println!(
        "dirty input:  {:?} (borrowed: {})",
        result2,
        matches!(result2, Cow::Borrowed(_))
    );

    // Cow<str> derefs to &str — usable anywhere &str is expected
    println!("result1 length: {}", result1.len());

    // --- From conversions ---
    let borrowed: Cow<str> = Cow::Borrowed("static text");
    let owned: Cow<str> = Cow::Owned(String::from("owned text"));
    println!("borrowed: {borrowed}, owned: {owned}");

    // Convenient From impls
    let from_ref: Cow<str> = "literal".into();
    let from_string: Cow<str> = String::from("dynamic").into();
    println!("from &str: {from_ref}, from String: {from_string}");

    // --- to_mut(): clone-on-write in action ---
    let mut cow: Cow<str> = Cow::Borrowed("hello");
    println!(
        "before to_mut: borrowed = {}",
        matches!(cow, Cow::Borrowed(_))
    );
    // First call to to_mut() clones the borrowed data into an owned String
    cow.to_mut().push_str(", world!");
    println!(
        "after to_mut:  borrowed = {}, value = {cow}",
        matches!(cow, Cow::Borrowed(_))
    );

    // --- Cow with slices ---
    fn ensure_sorted(data: &[i32]) -> Cow<'_, [i32]> {
        if data.windows(2).all(|w| w[0] <= w[1]) {
            Cow::Borrowed(data) // already sorted — no allocation
        } else {
            let mut sorted = data.to_vec();
            sorted.sort();
            Cow::Owned(sorted)
        }
    }

    let sorted_input = [1, 2, 3, 4, 5];
    let unsorted_input = [3, 1, 4, 1, 5];
    let r1 = ensure_sorted(&sorted_input);
    let r2 = ensure_sorted(&unsorted_input);
    println!(
        "sorted input:   {:?} (borrowed: {})",
        &*r1,
        matches!(r1, Cow::Borrowed(_))
    );
    println!(
        "unsorted input: {:?} (borrowed: {})",
        &*r2,
        matches!(r2, Cow::Borrowed(_))
    );

    // --- into_owned(): force ownership ---
    let cow: Cow<str> = Cow::Borrowed("borrow me");
    let owned_string: String = cow.into_owned(); // clones here
    println!("into_owned: {owned_string}");

    println!("cow_clone_on_write section executed");
}

// ============================================================
// pub fn run()
// ============================================================

pub fn run() {
    smart_pointers_and_ownership_control();
    supertraits_extension_traits_and_blanket_impls();
    marker_traits_and_wrapper_structs();
    marker_structs_and_phantom_types();
    struct_tagging_type_state_pattern();
    const_generics_and_compile_time_parameterization();
    cow_clone_on_write();
}
