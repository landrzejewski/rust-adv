#![allow(unused_imports, unused_mut, dead_code, unused_variables)]

use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::fmt;
use std::mem::size_of;

// C standard library functions — link automatically on all platforms
unsafe extern "C" {
    fn abs(n: c_int) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn atoi(s: *const c_char) -> c_int;
    fn sqrt(x: c_double) -> c_double;
    fn pow(base: c_double, exp: c_double) -> c_double;
    fn puts(s: *const c_char) -> c_int;
}

// ===========================================================================
// Section 0: FFI Theory
// ===========================================================================

fn ffi_theory() {
    println!("\n=== Section 0: FFI Theory ===\n");

    println!("FFI (Foreign Function Interface) lets Rust call code written in");
    println!("other languages — most commonly C, since the C calling convention");
    println!("is the universal standard for cross-language interop.\n");

    println!("Why FFI matters:");
    println!("  - Reuse established C libraries (SQLite, OpenSSL, zlib, libcurl)");
    println!("  - Access OS APIs (POSIX, Windows API) defined as C interfaces");
    println!("  - Gradual migration: rewrite C/C++ code in Rust module by module");
    println!("  - Leverage well-tested, performance-critical C code\n");

    println!("ABI (Application Binary Interface):");
    println!("  The low-level contract for how functions pass arguments, return");
    println!("  values, and manage the call stack. Two programs can only call each");
    println!("  other if they agree on the ABI.\n");

    println!("  extern \"C\"  — C calling convention (most widely supported)");
    println!("  extern \"system\" — stdcall on Windows, same as \"C\" elsewhere");
    println!("  extern \"Rust\" — default Rust ABI (unstable, never use for FFI)\n");

    println!("Five key C types from std::ffi:");
    println!("  c_int    — matches C int (typically i32)");
    println!("  c_double — matches C double (f64)");
    println!("  c_char   — matches C char (i8 or u8, platform-dependent)");
    println!("  c_void   — matches C void (used for opaque pointers)");
    println!("  c_uint   — matches C unsigned int (typically u32)\n");

    println!("Safety: ALL calls to foreign functions require `unsafe`.");
    println!("The compiler cannot verify foreign code's memory safety.");
    println!("Since Rust 2024, extern blocks must be `unsafe extern \"C\" {{ ... }}`.");
}

// ===========================================================================
// Section 1: Calling C Functions
// ===========================================================================

fn calling_c_from_rust() {
    println!("\n=== Section 1: Calling C Functions ===\n");

    // --- Step 1: abs() — simplest C call ---
    let result = unsafe { abs(-42) };
    let rust_result = (-42_i32).abs();
    println!("  Step 1 — C abs(-42) = {result}");
    println!("  Rust (-42_i32).abs() = {rust_result}");
    println!("  Both return 42, but the Rust version is verified at compile time");

    // --- Step 2: strlen() — pass CString to C ---
    // CString creates a null-terminated string suitable for C
    let hello = CString::new("hello").unwrap();
    let len = unsafe { strlen(hello.as_ptr()) };
    println!("\n  Step 2 — C strlen(\"hello\") = {len}");

    // --- Step 3: sqrt() and pow() — math functions ---
    let root = unsafe { sqrt(144.0) };
    let power = unsafe { pow(2.0, 10.0) };
    println!("\n  Step 3 — C sqrt(144.0) = {root}");
    println!("  C pow(2.0, 10.0) = {power}");

    // --- Step 4: atoi() — string to int ---
    // atoi returns 0 for non-numeric strings (no error reporting)
    let num_str = CString::new("42").unwrap();
    let num = unsafe { atoi(num_str.as_ptr()) };
    let bad_str = CString::new("not_a_number").unwrap();
    let bad_num = unsafe { atoi(bad_str.as_ptr()) };
    println!("\n  Step 4 — C atoi(\"42\") = {num}");
    println!("  C atoi(\"not_a_number\") = {bad_num} (0 = C's error-prone default)");
}

// ===========================================================================
// Section 2: C-Compatible Types
// ===========================================================================

// A C-compatible struct — field order and padding match C
#[repr(C)]
#[derive(Debug)]
struct Point2D {
    x: c_double,
    y: c_double,
}

// Nested C-compatible struct
#[repr(C)]
#[derive(Debug)]
struct Rect {
    origin: Point2D,
    width: c_double,
    height: c_double,
}

// C-compatible enum with explicit integer representation
#[repr(u32)]
#[derive(Debug, PartialEq)]
enum Color {
    Red = 0,
    Green = 1,
    Blue = 2,
}

fn c_compatible_types() {
    println!("\n=== Section 2: C-Compatible Types ===\n");

    // --- Step 5: repr(C) struct — guaranteed C-compatible layout ---
    let p = Point2D { x: 3.0, y: 4.0 };
    println!("  Step 5 — Point2D: {:?}", p);
    println!("  size: {} bytes", size_of::<Point2D>());
    println!(
        "  offsets: x={}, y={}",
        std::mem::offset_of!(Point2D, x),
        std::mem::offset_of!(Point2D, y)
    );

    // --- Step 6: Nested repr(C) struct ---
    let r = Rect {
        origin: Point2D { x: 0.0, y: 0.0 },
        width: 10.0,
        height: 5.0,
    };
    println!("\n  Step 6 — Rect: {:?}", r);
    println!("  size: {} bytes", size_of::<Rect>());
    println!(
        "  offsets: origin={}, width={}, height={}",
        std::mem::offset_of!(Rect, origin),
        std::mem::offset_of!(Rect, width),
        std::mem::offset_of!(Rect, height)
    );

    // --- Step 7: repr(u32) enum — explicit discriminant ---
    let color = Color::Blue;
    println!("\n  Step 7 — Color::Blue = {:?}, size: {} bytes", color, size_of::<Color>());

    // --- Step 8: offset_of! for field inspection ---
    #[repr(C)]
    struct CLayout {
        a: u8,  // 1 byte + 7 padding
        b: u64, // 8 bytes
        c: u8,  // 1 byte + 7 padding
    }
    println!("\n  Step 8 — CLayout (u8, u64, u8) with repr(C):");
    println!("  size: {} bytes (C-compatible, fields in declaration order)", size_of::<CLayout>());
    println!(
        "  offsets: a={}, b={}, c={}",
        std::mem::offset_of!(CLayout, a),
        std::mem::offset_of!(CLayout, b),
        std::mem::offset_of!(CLayout, c)
    );

    // --- Step 9: Type correspondence table ---
    println!("\n  Step 9 — Portable C type sizes on this platform:");
    println!("  c_int = {} bytes, c_double = {} bytes", size_of::<c_int>(), size_of::<c_double>());
    println!("  c_char = {} byte, *const c_void = {} bytes", size_of::<c_char>(), size_of::<*const c_void>());
    println!("\n  Type correspondence:");
    println!("    Rust i32/u32 ↔ C int32_t/uint32_t");
    println!("    Rust f64     ↔ C double");
    println!("    Rust bool    ↔ C _Bool (but many C APIs use int for booleans)");
    println!("    Rust char (4B) ≠ C char (1B) — use c_char for FFI");
}

// ===========================================================================
// Section 3: Strings Across FFI
// ===========================================================================

fn strings_across_ffi() {
    println!("\n=== Section 3: Strings Across FFI ===\n");

    // --- Step 10: CString::new() for Rust→C ---
    // Always bind CString to a variable — temporaries get dropped!
    let greeting = CString::new("Hello from Rust via FFI!").unwrap();
    let len = unsafe { strlen(greeting.as_ptr()) };
    println!("  Step 10 — CString: Rust→C string passing");
    println!("  C strlen = {len}");

    // --- Step 11: CStr::from_ptr() for C→Rust ---
    let original = CString::new("round trip").unwrap();
    let recovered = unsafe {
        // SAFETY: ptr points to a valid, null-terminated CString
        CStr::from_ptr(original.as_ptr()).to_str().unwrap()
    };
    println!("\n  Step 11 — CStr: C→Rust string reading");
    println!("  recovered: \"{recovered}\"");

    // --- Step 12: Interior null rejection ---
    let bad = CString::new("hello\0world");
    println!("\n  Step 12 — CString rejects interior nulls:");
    println!("  CString::new(\"hello\\0world\") = {bad:?}");

    // --- Step 13: CString::into_raw() / from_raw() ownership transfer ---
    let greeting = CString::new("hello from Rust").unwrap();
    let raw_ptr: *mut c_char = greeting.into_raw();
    // Rust no longer manages this memory — it could be passed to C
    let reclaimed = unsafe { CString::from_raw(raw_ptr) };
    println!("\n  Step 13 — Ownership transfer with into_raw/from_raw:");
    println!("  reclaimed: {:?}", reclaimed.to_str().unwrap());

    // --- Step 14: c"..." literal + to_string_lossy ---
    let literal: &CStr = c"compile-time C string";
    println!("\n  Step 14 — c\"...\" literal (no allocation, &'static CStr):");
    println!("  bytes: {:?}", literal.to_bytes());

    let bytes: &[u8] = b"valid ascii\0";
    let c_str = CStr::from_bytes_with_nul(bytes).unwrap();
    println!("  to_string_lossy: {}", c_str.to_string_lossy());
}

// ===========================================================================
// Section 4: Exporting Rust to C
// ===========================================================================

// A struct intended to be created in Rust and used by C callers
#[repr(C)]
pub struct RustVec2 {
    x: c_double,
    y: c_double,
}

impl RustVec2 {
    fn magnitude(&self) -> c_double {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl fmt::Display for RustVec2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

// Corresponding C header (cbindgen would generate this):
//   typedef struct RustVec2 { double x; double y; } RustVec2;
//   RustVec2* tut_vec2_new(double x, double y);
//   double tut_vec2_magnitude(const RustVec2* ptr);
//   void tut_vec2_free(RustVec2* ptr);

#[unsafe(no_mangle)]
pub extern "C" fn tut_vec2_new(x: c_double, y: c_double) -> *mut RustVec2 {
    Box::into_raw(Box::new(RustVec2 { x, y }))
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn tut_vec2_magnitude(ptr: *const RustVec2) -> c_double {
    if ptr.is_null() {
        return -1.0; // error sentinel
    }
    // SAFETY: caller guarantees ptr was created by tut_vec2_new
    unsafe { (*ptr).magnitude() }
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn tut_vec2_free(ptr: *mut RustVec2) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: ptr was created by tut_vec2_new via Box::into_raw
    unsafe {
        drop(Box::from_raw(ptr));
    }
}

fn exporting_rust_to_c() {
    println!("\n=== Section 4: Exporting Rust to C ===\n");

    // --- Step 15: #[unsafe(no_mangle)] + extern "C" fn ---
    println!("  Step 15 — #[unsafe(no_mangle)] pub extern \"C\" fn");
    println!("  no_mangle preserves the symbol name for C to find");
    println!("  extern \"C\" uses the C calling convention");

    // --- Step 16: Box::into_raw for heap allocation ---
    let ptr = tut_vec2_new(3.0, 4.0);
    let mag = tut_vec2_magnitude(ptr);
    println!("\n  Step 16 — Box::into_raw: allocate for C consumption");
    println!("  tut_vec2_new(3.0, 4.0) → magnitude = {mag}");

    // --- Step 17: Box::from_raw for deallocation ---
    tut_vec2_free(ptr);
    println!("\n  Step 17 — Box::from_raw: reclaim and deallocate");
    println!("  tut_vec2_free(ptr) — memory freed by Rust's allocator");

    // --- Step 18: Null pointer checks ---
    tut_vec2_free(std::ptr::null_mut()); // safe no-op
    let err = tut_vec2_magnitude(std::ptr::null());
    println!("\n  Step 18 — Null pointer safety:");
    println!("  tut_vec2_free(null) = no-op");
    println!("  tut_vec2_magnitude(null) = {err} (error sentinel)");
}

// ===========================================================================
// Section 5: Callbacks
// ===========================================================================

// A callback type matching C convention
type CCallback = extern "C" fn(c_int) -> c_int;

// C-style callback with user_data void pointer
type CCallbackWithData = extern "C" fn(c_int, *mut c_void) -> c_int;

// Rust functions with C calling convention, usable as callbacks
extern "C" fn double_it(x: c_int) -> c_int {
    x * 2
}

extern "C" fn square_it(x: c_int) -> c_int {
    x * x
}

extern "C" fn add_offset(x: c_int, user_data: *mut c_void) -> c_int {
    // SAFETY: caller guarantees user_data points to a valid c_int
    let offset = unsafe { *(user_data as *const c_int) };
    x + offset
}

fn apply_callback(values: &[c_int], callback: CCallback) -> Vec<c_int> {
    values.iter().map(|&v| callback(v)).collect()
}

fn apply_with_data(values: &[c_int], cb: CCallbackWithData, data: *mut c_void) -> Vec<c_int> {
    values.iter().map(|&v| cb(v, data)).collect()
}

fn callbacks_and_function_pointers() {
    println!("\n=== Section 5: Callbacks ===\n");

    // --- Step 19: Basic extern "C" fn callbacks ---
    let data = [1, 2, 3, 4, 5];
    let doubled = apply_callback(&data, double_it);
    let squared = apply_callback(&data, square_it);
    println!("  Step 19 — Basic C-style callbacks:");
    println!("  doubled: {doubled:?}");
    println!("  squared: {squared:?}");

    // --- Step 20: apply_callback with function pointer parameter ---
    // Any extern "C" fn matching the signature can be passed
    let custom: CCallback = double_it;
    let result = apply_callback(&[10, 20], custom);
    println!("\n  Step 20 — Function pointer as parameter:");
    println!("  apply_callback([10, 20], double_it) = {result:?}");

    // --- Step 21: Option<extern "C" fn> niche optimization ---
    let some_cb: Option<CCallback> = Some(double_it);
    let none_cb: Option<CCallback> = None;
    println!("\n  Step 21 — Nullable function pointer (zero-cost):");
    println!(
        "  size of CCallback: {}, Option<CCallback>: {} (same!)",
        size_of::<CCallback>(),
        size_of::<Option<CCallback>>()
    );
    if let Some(cb) = some_cb {
        println!("  some_cb(5) = {}", cb(5));
    }
    println!("  none_cb is None = {} (null in C terms)", none_cb.is_none());

    // --- Step 22: void* user_data pattern ---
    let mut offset: c_int = 100;
    let data_ptr: *mut c_void = &mut offset as *mut c_int as *mut c_void;
    let with_offset = apply_with_data(&[1, 2, 3], add_offset, data_ptr);
    println!("\n  Step 22 — C-style user_data pattern:");
    println!("  with offset {offset}: {with_offset:?}");
    println!("  (The callback casts void* back to the expected type)");
}

// ===========================================================================
// Section 6: Build Tools
// ===========================================================================

fn build_tools() {
    println!("\n=== Section 6: Build Tools ===\n");

    println!("  FFI Build Tools Reference:");
    println!("  ─────────────────────────────────────────────────────");
    println!("  build.rs         Cargo build script, runs before compilation");
    println!("  cc crate         Compile C/C++ files from build.rs");
    println!("  bindgen          Generate Rust bindings from C headers (.h → .rs)");
    println!("  cbindgen         Generate C headers from Rust code (.rs → .h)");
    println!();
    println!("  crate-type in Cargo.toml [lib]:");
    println!("    \"cdylib\"       C-compatible dynamic library (.so/.dylib/.dll)");
    println!("    \"staticlib\"    Static library (.a/.lib)");
    println!();
    println!("  Example build.rs with cc crate:");
    println!("    fn main() {{");
    println!("        cc::Build::new()");
    println!("            .file(\"c_src/helper.c\")");
    println!("            .compile(\"helper\");");
    println!("    }}");
    println!();
    println!("  Example Cargo.toml:");
    println!("    [lib]");
    println!("    crate-type = [\"cdylib\"]");
    println!("    [build-dependencies]");
    println!("    cc = \"1.0\"");
}

// ===========================================================================
// Section 7: Safe Wrappers
// ===========================================================================

// Simulated C library: a "database handle"
#[repr(C)]
struct RawDbHandle {
    name: *mut c_char,
    query_count: c_int,
    connected: bool,
}

extern "C" fn db_open(name: *const c_char) -> *mut RawDbHandle {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let name_owned = unsafe { CStr::from_ptr(name) };
    let name_copy = CString::new(name_owned.to_bytes()).unwrap();
    Box::into_raw(Box::new(RawDbHandle {
        name: name_copy.into_raw(),
        query_count: 0,
        connected: true,
    }))
}

extern "C" fn db_execute(handle: *mut RawDbHandle, _query: *const c_char) -> c_int {
    if handle.is_null() {
        return -1;
    }
    unsafe {
        if !(*handle).connected {
            return -2;
        }
        (*handle).query_count += 1;
    }
    0
}

extern "C" fn db_close(handle: *mut RawDbHandle) {
    if handle.is_null() {
        return;
    }
    unsafe {
        (*handle).connected = false;
        if !(*handle).name.is_null() {
            drop(CString::from_raw((*handle).name));
        }
        drop(Box::from_raw(handle));
    }
}

// Safe RAII wrapper — users never write `unsafe`
struct Database {
    handle: *mut RawDbHandle,
}

impl Database {
    fn open(name: &str) -> Result<Self, &'static str> {
        let c_name = CString::new(name).map_err(|_| "name contains null byte")?;
        let handle = db_open(c_name.as_ptr());
        if handle.is_null() {
            return Err("failed to open database");
        }
        Ok(Self { handle })
    }

    fn execute(&mut self, query: &str) -> Result<(), &'static str> {
        let c_query = CString::new(query).map_err(|_| "query contains null byte")?;
        let result = db_execute(self.handle, c_query.as_ptr());
        if result != 0 {
            Err("query execution failed")
        } else {
            Ok(())
        }
    }

    fn name(&self) -> &str {
        unsafe {
            let name_ptr = (*self.handle).name;
            if name_ptr.is_null() {
                "<unknown>"
            } else {
                CStr::from_ptr(name_ptr)
                    .to_str()
                    .unwrap_or("<invalid utf-8>")
            }
        }
    }

    fn query_count(&self) -> i32 {
        unsafe { (*self.handle).query_count }
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        db_close(self.handle);
    }
}

fn safe_wrappers() {
    println!("\n=== Section 7: Safe Wrappers ===\n");

    // --- Step 23: Simulated C functions ---
    println!("  Step 23 — Simulated C library (db_open, db_execute, db_close)");
    println!("  These represent raw C functions with pointer-based interfaces");

    // --- Step 24: Database safe wrapper with RAII Drop ---
    println!("\n  Step 24 — Safe RAII wrapper encapsulates all unsafe:");
    {
        let mut db = Database::open("my_app.db").unwrap();
        db.execute("SELECT * FROM users").unwrap();
        db.execute("INSERT INTO logs VALUES (...)").unwrap();

        // --- Step 25: Safe methods ---
        println!("\n  Step 25 — Safe methods: name(), query_count():");
        println!(
            "  database: {}, queries executed: {}",
            db.name(),
            db.query_count()
        );

        // --- Step 26: Automatic cleanup ---
        // db dropped here → db_close called automatically
    }
    println!("\n  Step 26 — database closed automatically via Drop");

    // Error handling works naturally through Result
    let bad = Database::open("has\0null");
    println!("  open with null in name: {:?}", bad.err());
}

// ===========================================================================
// Section 8: Safety Pitfalls
// ===========================================================================

fn safety_pitfalls() {
    println!("\n=== Section 8: Safety Pitfalls ===\n");

    // --- Step 27: catch_unwind at FFI boundaries ---
    extern "C" fn panicking_callback() -> c_int {
        let result = std::panic::catch_unwind(|| -> c_int {
            panic!("oops! this would be UB without catch_unwind");
        });
        match result {
            Ok(val) => val,
            Err(_) => -1, // convert panic to error code
        }
    }
    let result = panicking_callback();
    println!("  Step 27 — Panic in extern \"C\" fn aborts the process; use catch_unwind:");
    println!("  caught panic → error code = {result}");

    // --- Step 28: extern "C-unwind" ---
    println!("\n  Step 28 — extern \"C-unwind\" (Rust 1.71+):");
    println!("  Allows unwinding across C ABI boundaries");
    println!("  Use for Rust→C callback→Rust chains");
    println!("  Do NOT use when the caller is actual C code");

    // --- Step 29: Opaque type pattern ---
    #[repr(C)]
    struct OpaqueHandle {
        _private: [u8; 0],
    }
    fn create_handle() -> *mut OpaqueHandle {
        std::ptr::null_mut() // placeholder
    }
    let handle = create_handle();
    println!("\n  Step 29 — Opaque type pattern (zero-sized):");
    println!("  OpaqueHandle {{ _private: [u8; 0] }} — cannot be constructed");
    println!("  Forces all interaction through raw pointers: {:p}", handle);

    // --- Step 30: Safety checklist ---
    println!("\n  Step 30 — FFI Safety Checklist:");
    println!("  □ Use #[repr(C)] on all types crossing FFI boundaries");
    println!("  □ Check for null before dereferencing pointers");
    println!("  □ Whoever allocates must deallocate (don't mix allocators)");
    println!("  □ Keep CString alive while its pointer is in use");
    println!("  □ Use catch_unwind in extern \"C\" fn called from C");
    println!("  □ Use c_int/c_double, not i32/f64, in FFI signatures");
    println!("  □ Wrap unsafe FFI in safe Rust APIs with RAII Drop");
    println!("  □ Document safety invariants with #Safety comments");
}

// ===========================================================================
// Public entry point
// ===========================================================================

pub fn run() {
    ffi_theory();
    calling_c_from_rust();
    c_compatible_types();
    strings_across_ffi();
    exporting_rust_to_c();
    callbacks_and_function_pointers();
    build_tools();
    safe_wrappers();
    safety_pitfalls();
}
