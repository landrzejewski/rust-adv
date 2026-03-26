#![allow(unused_imports, unused_mut, dead_code, unused_variables)]

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ===========================================================================
// Simulation Layer — PyO3-like types for runnable examples
// ===========================================================================

// Simulated Python exception — mirrors pyo3::PyErr
#[derive(Debug, Clone)]
pub struct PyErr {
    pub exception_type: &'static str,
    pub message: String,
}

impl PyErr {
    pub fn new<E: ExceptionType>(msg: impl Into<String>) -> Self {
        PyErr {
            exception_type: E::NAME,
            message: msg.into(),
        }
    }
}

impl fmt::Display for PyErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.exception_type, self.message)
    }
}

pub type PyResult<T> = Result<T, PyErr>;

pub trait ExceptionType {
    const NAME: &'static str;
}

pub struct ValueError;
impl ExceptionType for ValueError {
    const NAME: &'static str = "ValueError";
}

pub struct TypeError;
impl ExceptionType for TypeError {
    const NAME: &'static str = "TypeError";
}

pub struct RuntimeError;
impl ExceptionType for RuntimeError {
    const NAME: &'static str = "RuntimeError";
}

pub struct ZeroDivisionError;
impl ExceptionType for ZeroDivisionError {
    const NAME: &'static str = "ZeroDivisionError";
}

pub struct OverflowError;
impl ExceptionType for OverflowError {
    const NAME: &'static str = "OverflowError";
}

pub struct PanicException;
impl ExceptionType for PanicException {
    const NAME: &'static str = "PanicException";
}

// Simulated Python object — mirrors pyo3::types::PyAny
#[derive(Debug, Clone)]
pub enum PyAny {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    List(Vec<PyAny>),
    None,
}

impl fmt::Display for PyAny {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PyAny::Int(v) => write!(f, "{v}"),
            PyAny::Float(v) => write!(f, "{v}"),
            PyAny::Str(v) => write!(f, "'{v}'"),
            PyAny::Bool(v) => {
                if *v { write!(f, "True") } else { write!(f, "False") }
            }
            PyAny::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            PyAny::None => write!(f, "None"),
        }
    }
}

// Simulated FromPyObject — extract a Rust type from a Python object
pub trait FromPyObject: Sized {
    fn extract(obj: &PyAny) -> PyResult<Self>;
}

// Simulated IntoPyObject — convert a Rust type to a Python object
pub trait IntoPyObject {
    fn into_py(self) -> PyAny;
}

// Simulated GIL token
pub struct Python {
    _lock: Arc<Mutex<()>>,
}

impl Python {
    pub fn with_gil<F, R>(f: F) -> R
    where
        F: FnOnce(&Python) -> R,
    {
        let py = Python {
            _lock: Arc::new(Mutex::new(())),
        };
        f(&py)
    }

    pub fn allow_threads<F, R>(&self, f: F) -> R
    where
        F: Send + FnOnce() -> R,
    {
        f()
    }
}

// Simulated module — represents a Python module with registered functions
pub struct SimModule {
    name: String,
    functions: Vec<(String, Box<dyn Fn(Vec<PyAny>) -> PyResult<PyAny>>)>,
}

impl SimModule {
    pub fn new(name: &str) -> Self {
        SimModule {
            name: name.to_string(),
            functions: Vec::new(),
        }
    }

    pub fn add_function(&mut self, name: &str, func: Box<dyn Fn(Vec<PyAny>) -> PyResult<PyAny>>) {
        self.functions.push((name.to_string(), func));
    }

    pub fn call(&self, name: &str, args: Vec<PyAny>) -> PyResult<PyAny> {
        for (fname, func) in &self.functions {
            if fname == name {
                return func(args);
            }
        }
        Err(PyErr::new::<RuntimeError>(format!(
            "module '{}' has no function '{name}'",
            self.name
        )))
    }
}

fn type_name(obj: &PyAny) -> &'static str {
    match obj {
        PyAny::Int(_) => "int",
        PyAny::Float(_) => "float",
        PyAny::Str(_) => "str",
        PyAny::Bool(_) => "bool",
        PyAny::List(_) => "list",
        PyAny::None => "NoneType",
    }
}

// ===========================================================================
// FromPyObject implementations
// ===========================================================================

impl FromPyObject for i64 {
    fn extract(obj: &PyAny) -> PyResult<Self> {
        match obj {
            PyAny::Int(v) => Ok(*v),
            PyAny::Bool(v) => Ok(if *v { 1 } else { 0 }),
            other => Err(PyErr::new::<TypeError>(format!(
                "cannot convert {} to int", type_name(other)
            ))),
        }
    }
}

impl FromPyObject for f64 {
    fn extract(obj: &PyAny) -> PyResult<Self> {
        match obj {
            PyAny::Float(v) => Ok(*v),
            PyAny::Int(v) => Ok(*v as f64),
            other => Err(PyErr::new::<TypeError>(format!(
                "cannot convert {} to float", type_name(other)
            ))),
        }
    }
}

impl FromPyObject for String {
    fn extract(obj: &PyAny) -> PyResult<Self> {
        match obj {
            PyAny::Str(v) => Ok(v.clone()),
            other => Err(PyErr::new::<TypeError>(format!(
                "cannot convert {} to str", type_name(other)
            ))),
        }
    }
}

impl FromPyObject for bool {
    fn extract(obj: &PyAny) -> PyResult<Self> {
        match obj {
            PyAny::Bool(v) => Ok(*v),
            PyAny::Int(v) => Ok(*v != 0),
            other => Err(PyErr::new::<TypeError>(format!(
                "cannot convert {} to bool", type_name(other)
            ))),
        }
    }
}

impl FromPyObject for Vec<i64> {
    fn extract(obj: &PyAny) -> PyResult<Self> {
        match obj {
            PyAny::List(items) => items.iter().map(|item| i64::extract(item)).collect(),
            other => Err(PyErr::new::<TypeError>(format!(
                "cannot convert {} to list[int]", type_name(other)
            ))),
        }
    }
}

impl FromPyObject for Vec<PyAny> {
    fn extract(obj: &PyAny) -> PyResult<Self> {
        match obj {
            PyAny::List(items) => Ok(items.clone()),
            other => Err(PyErr::new::<TypeError>(format!(
                "cannot convert {} to list", type_name(other)
            ))),
        }
    }
}

// ===========================================================================
// IntoPyObject implementations
// ===========================================================================

impl IntoPyObject for i64 {
    fn into_py(self) -> PyAny { PyAny::Int(self) }
}

impl IntoPyObject for f64 {
    fn into_py(self) -> PyAny { PyAny::Float(self) }
}

impl IntoPyObject for String {
    fn into_py(self) -> PyAny { PyAny::Str(self) }
}

impl IntoPyObject for bool {
    fn into_py(self) -> PyAny { PyAny::Bool(self) }
}

impl IntoPyObject for Vec<i64> {
    fn into_py(self) -> PyAny {
        PyAny::List(self.into_iter().map(|v| PyAny::Int(v)).collect())
    }
}

impl<T: IntoPyObject> IntoPyObject for Option<T> {
    fn into_py(self) -> PyAny {
        match self {
            Some(v) => v.into_py(),
            None => PyAny::None,
        }
    }
}

// ===========================================================================
// Section 0: Why Python-Rust Interop
// ===========================================================================

fn why_python_rust() {
    println!("\n=== Section 0: Why Python-Rust Interop ===\n");

    println!("Python is dominant in data science, ML, and scripting — but:");
    println!("  - Interpreted execution is 10-100x slower for CPU-bound work");
    println!("  - The GIL prevents true multi-threaded parallelism");
    println!("  - High memory overhead per object\n");

    println!("Solution: write hot paths in Rust, expose them to Python.\n");

    println!("Success stories:");
    println!("  Polars     — DataFrame lib, 10-100x faster than pandas");
    println!("  Pydantic v2 — validation, 5-50x faster than pure-Python v1");
    println!("  ruff       — Python linter, 10-100x faster than flake8");
    println!("  uv         — package installer, 10-100x faster than pip");
    println!("  cryptography — Python's most popular crypto lib (Rust core)");
    println!("  tiktoken   — OpenAI's tokenizer (Rust core)\n");

    // Quick benchmark: sum-of-squares
    let n: i64 = 1_000_000;
    let start = Instant::now();
    let rust_result: i64 = (1..=n).map(|x| x * x).sum();
    let rust_time = start.elapsed();

    let start = Instant::now();
    let boxed_result: i64 = (1..=n)
        .map(|x| {
            let boxed = Box::new(x); // simulate Python int boxing
            let val = *boxed;
            val * val
        })
        .sum();
    let boxed_time = start.elapsed();

    println!("  Sum of squares (1 to {n}):");
    println!("    Rust  : {rust_result} in {rust_time:?}");
    println!("    Boxed : {boxed_result} in {boxed_time:?}");
    println!("    (Real CPython would be ~50-100x slower than Rust)\n");

    println!("  Approach  | Mechanism               | Best For");
    println!("  ----------|-------------------------|---------------------------");
    println!("  PyO3      | Native extension module | New Rust+Python projects");
    println!("  cffi      | C ABI + Python cffi lib | Existing C-ABI libraries");
    println!("  ctypes    | C ABI + Python ctypes   | Quick prototyping");
}

// ===========================================================================
// Section 1: PyO3 Fundamentals
// ===========================================================================

fn pyo3_fundamentals() {
    println!("\n=== Section 1: PyO3 Fundamentals ===\n");

    // --- Step 1: SimModule creation and function registration ---
    let mut module = SimModule::new("my_module");

    module.add_function(
        "sum_as_string",
        Box::new(|args: Vec<PyAny>| -> PyResult<PyAny> {
            if args.len() != 2 {
                return Err(PyErr::new::<TypeError>("expected 2 arguments"));
            }
            let a = i64::extract(&args[0])?;
            let b = i64::extract(&args[1])?;
            Ok(PyAny::Str((a + b).to_string()))
        }),
    );

    module.add_function(
        "multiply",
        Box::new(|args: Vec<PyAny>| -> PyResult<PyAny> {
            if args.len() != 2 {
                return Err(PyErr::new::<TypeError>("expected 2 arguments"));
            }
            let a = f64::extract(&args[0])?;
            let b = f64::extract(&args[1])?;
            Ok(PyAny::Float(a * b))
        }),
    );

    println!("  Step 1 — Module creation and function registration:");
    println!("  Created module 'my_module' with sum_as_string, multiply");

    // --- Step 2: Calling registered functions ---
    let result = module.call("sum_as_string", vec![PyAny::Int(5), PyAny::Int(3)]);
    println!("\n  Step 2 — Calling through module dispatch:");
    println!("  sum_as_string(5, 3) = {}", result.unwrap());

    let result = module.call("multiply", vec![PyAny::Float(3.14), PyAny::Float(2.0)]);
    println!("  multiply(3.14, 2.0) = {}", result.unwrap());

    // --- Step 3: Error on missing function ---
    let result = module.call("nonexistent", vec![]);
    println!("\n  Step 3 — Error on missing function:");
    println!("  nonexistent() = {}", result.unwrap_err());

    // --- Step 4: PyO3 attribute summary ---
    println!("\n  Step 4 — Key PyO3 attributes (real code, not simulated):");
    println!("  #[pyfunction]          — expose Rust fn to Python");
    println!("  #[pymodule]            — module entry point");
    println!("  wrap_pyfunction!(f, m) — register function in module");
    println!("  #[pyo3(signature = (a, b=42))] — default arguments");
}

// ===========================================================================
// Section 2: Type Conversions
// ===========================================================================

fn type_conversions() {
    println!("\n=== Section 2: Type Conversions ===\n");

    // --- Step 5: FromPyObject extractions ---
    println!("  Step 5 — FromPyObject (Python → Rust):");
    let rust_int: i64 = i64::extract(&PyAny::Int(42)).unwrap();
    println!("  Int(42) → i64: {rust_int}");

    let rust_float: f64 = f64::extract(&PyAny::Float(3.14)).unwrap();
    println!("  Float(3.14) → f64: {rust_float}");

    let rust_str: String = String::extract(&PyAny::Str("hello".into())).unwrap();
    println!("  Str('hello') → String: {rust_str}");

    let rust_bool: bool = bool::extract(&PyAny::Bool(true)).unwrap();
    println!("  Bool(True) → bool: {rust_bool}");

    // --- Step 6: Coercion (Int→f64 works, like Python) ---
    let coerced: f64 = f64::extract(&PyAny::Int(7)).unwrap();
    let bool_as_int: i64 = i64::extract(&PyAny::Bool(true)).unwrap();
    println!("\n  Step 6 — Coercions (Python-like):");
    println!("  Int(7) → f64: {coerced}");
    println!("  Bool(True) → i64: {bool_as_int}");

    // --- Step 7: Type mismatch errors ---
    println!("\n  Step 7 — Type mismatch errors:");
    println!("  Str → i64: {}", i64::extract(&PyAny::Str("nope".into())).unwrap_err());
    println!("  Int → String: {}", String::extract(&PyAny::Int(42)).unwrap_err());

    // --- Step 8: IntoPyObject (Rust → Python) ---
    println!("\n  Step 8 — IntoPyObject (Rust → Python):");
    println!("  42_i64.into_py() = {}", 42_i64.into_py());
    println!("  \"hello\".into_py() = {}", "hello".to_string().into_py());
    println!("  vec![1, 2, 3].into_py() = {}", vec![1_i64, 2, 3].into_py());

    // --- Step 9: Option<T> handling ---
    let some_val: Option<i64> = Some(42);
    let none_val: Option<i64> = None;
    println!("\n  Step 9 — Option<T> conversion:");
    println!("  Some(42).into_py() = {}", some_val.into_py());
    println!("  None.into_py() = {}", none_val.into_py());
}

// ===========================================================================
// Section 3: Exposing Structs as Classes
// ===========================================================================

// Simulated #[pyclass]
#[derive(Clone, Debug)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn py_new(x: f64, y: f64) -> Self { Point { x, y } }
    fn get_x(&self) -> f64 { self.x }
    fn get_y(&self) -> f64 { self.y }
    fn set_x(&mut self, val: f64) { self.x = val; }

    fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    fn repr(&self) -> String {
        format!("Point(x={}, y={})", self.x, self.y)
    }

    fn str(&self) -> String {
        format!("({}, {})", self.x, self.y)
    }

    fn eq(&self, other: &Point) -> bool {
        self.x == other.x && self.y == other.y
    }

    fn into_pyany(&self) -> PyAny {
        PyAny::List(vec![PyAny::Float(self.x), PyAny::Float(self.y)])
    }
}

// Simulated #[pyclass] with validation
#[derive(Debug)]
struct PyRect {
    origin: Point,
    width: f64,
    height: f64,
}

impl PyRect {
    fn py_new(x: f64, y: f64, width: f64, height: f64) -> PyResult<Self> {
        if width < 0.0 || height < 0.0 {
            return Err(PyErr::new::<ValueError>(
                "width and height must be non-negative",
            ));
        }
        Ok(PyRect {
            origin: Point::py_new(x, y),
            width,
            height,
        })
    }

    fn area(&self) -> f64 { self.width * self.height }

    fn contains(&self, point: &Point) -> bool {
        point.x >= self.origin.x
            && point.x <= self.origin.x + self.width
            && point.y >= self.origin.y
            && point.y <= self.origin.y + self.height
    }

    fn repr(&self) -> String {
        format!(
            "Rect(origin={}, width={}, height={})",
            self.origin.str(), self.width, self.height
        )
    }
}

fn exposing_structs() {
    println!("\n=== Section 3: Exposing Structs as Classes ===\n");

    // --- Step 10: Point construction and getters ---
    let mut p = Point::py_new(3.0, 4.0);
    println!("  Step 10 — Construction and getters:");
    println!("  repr(p) = {}", p.repr());
    println!("  p.x = {}, p.y = {}", p.get_x(), p.get_y());

    // --- Step 11: Setter and distance ---
    p.set_x(5.0);
    let origin = Point::py_new(0.0, 0.0);
    let dist = p.distance(&origin);
    println!("\n  Step 11 — Setter and methods:");
    println!("  after p.x = 5.0: {}", p.repr());
    println!("  p.distance(origin) = {dist:.4}");

    // --- Step 12: repr/str/equality (dunder methods) ---
    let p1 = Point::py_new(1.0, 2.0);
    let p2 = Point::py_new(1.0, 2.0);
    let p3 = Point::py_new(3.0, 4.0);
    println!("\n  Step 12 — Dunder methods (__repr__, __str__, __eq__):");
    println!("  str(p1) = {}", p1.str());
    println!("  Point(1,2) == Point(1,2): {}", p1.eq(&p2));
    println!("  Point(1,2) == Point(3,4): {}", p1.eq(&p3));

    // --- Step 13: Rect with constructor validation ---
    let rect = PyRect::py_new(0.0, 0.0, 10.0, 5.0).unwrap();
    println!("\n  Step 13 — Rect with validation:");
    println!("  {}", rect.repr());
    let bad_rect = PyRect::py_new(0.0, 0.0, -1.0, 5.0);
    println!("  Rect(0, 0, -1, 5) = {}", bad_rect.unwrap_err());

    // --- Step 14: Rect area + contains ---
    println!("\n  Step 14 — Rect methods:");
    println!("  rect.area() = {}", rect.area());
    println!(
        "  rect.contains(Point(5, 3)) = {}",
        rect.contains(&Point::py_new(5.0, 3.0))
    );
    println!(
        "  rect.contains(Point(15, 3)) = {}",
        rect.contains(&Point::py_new(15.0, 3.0))
    );
}

// ===========================================================================
// Section 4: Error Handling
// ===========================================================================

// Custom error type with From<MyError> for PyErr
#[derive(Debug)]
enum MyError {
    InvalidInput(String),
    NotFound(String),
    ComputationFailed,
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MyError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            MyError::NotFound(msg) => write!(f, "not found: {msg}"),
            MyError::ComputationFailed => write!(f, "computation failed"),
        }
    }
}

impl From<MyError> for PyErr {
    fn from(e: MyError) -> PyErr {
        match e {
            MyError::InvalidInput(msg) => PyErr::new::<ValueError>(msg),
            MyError::NotFound(msg) => PyErr::new::<RuntimeError>(format!("not found: {msg}")),
            MyError::ComputationFailed => {
                PyErr::new::<RuntimeError>("computation failed".to_string())
            }
        }
    }
}

fn py_divide(a: f64, b: f64) -> PyResult<f64> {
    if b == 0.0 {
        Err(PyErr::new::<ZeroDivisionError>("division by zero"))
    } else {
        Ok(a / b)
    }
}

fn py_lookup(data: &HashMap<String, i64>, key: &str) -> PyResult<i64> {
    let value = data.get(key).ok_or(MyError::NotFound(key.to_string()))?;
    Ok(*value)
}

fn py_parse_and_validate(input: &str) -> PyResult<i64> {
    let value: i64 = input
        .parse()
        .map_err(|_| PyErr::new::<ValueError>(format!("cannot parse '{input}' as integer")))?;
    if value < 0 {
        return Err(PyErr::new::<ValueError>("value must be non-negative"));
    }
    if value > 1000 {
        return Err(PyErr::new::<OverflowError>(format!(
            "value {value} exceeds maximum of 1000"
        )));
    }
    Ok(value)
}

fn error_handling() {
    println!("\n=== Section 4: Error Handling ===\n");

    // --- Step 15: PyResult with standard exceptions ---
    println!("  Step 15 — PyResult with standard exceptions:");
    println!("  divide(10, 3) = {:?}", py_divide(10.0, 3.0));
    println!("  divide(10, 0) = {}", py_divide(10.0, 0.0).unwrap_err());

    // --- Step 16: Custom error type with From conversion ---
    let mut data = HashMap::new();
    data.insert("port".to_string(), 8080);
    data.insert("timeout".to_string(), 30);
    println!("\n  Step 16 — Custom error type with From<MyError> for PyErr:");
    println!("  lookup('port') = {:?}", py_lookup(&data, "port"));
    println!("  lookup('missing') = {}", py_lookup(&data, "missing").unwrap_err());

    // --- Step 17: Validation chain with ? ---
    println!("\n  Step 17 — Validation chains with ? operator:");
    println!("  parse('42') = {:?}", py_parse_and_validate("42"));
    println!("  parse('abc') = {}", py_parse_and_validate("abc").unwrap_err());
    println!("  parse('-5') = {}", py_parse_and_validate("-5").unwrap_err());
    println!("  parse('9999') = {}", py_parse_and_validate("9999").unwrap_err());

    // --- Step 18: Panic catch at FFI boundary ---
    println!("\n  Step 18 — Panic handling (PyO3 does this automatically):");
    let result = std::panic::catch_unwind(|| -> PyResult<i64> {
        panic!("unexpected error in Rust code!");
    });
    match result {
        Ok(py_result) => println!("  result: {py_result:?}"),
        Err(_) => {
            let err = PyErr::new::<PanicException>("a Rust panic occurred");
            println!("  caught panic → {err}");
        }
    }

    // --- Step 19: Error mapping guidance ---
    println!("\n  Step 19 — Error mapping guidance:");
    println!("  Rust std::io::Error      → PyIOError");
    println!("  Rust ParseIntError       → PyValueError");
    println!("  Custom enum variants     → implement From<MyError> for PyErr");
    println!("  Unknown/unexpected       → PyRuntimeError");
}

// ===========================================================================
// Section 5: The GIL
// ===========================================================================

fn the_gil() {
    println!("\n=== Section 5: The GIL ===\n");

    // --- Step 20: GIL contention simulation ---
    println!("  Step 20 — GIL contention (4 threads competing for Mutex):");
    let gil = Arc::new(Mutex::new(()));
    let results = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];
    for thread_id in 0..4 {
        let gil = Arc::clone(&gil);
        let results = Arc::clone(&results);
        handles.push(std::thread::spawn(move || {
            let _guard = gil.lock().unwrap();
            let value = thread_id * 10;
            results.lock().unwrap().push((thread_id, value));
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    let mut results_vec = results.lock().unwrap().clone();
    results_vec.sort_by_key(|&(id, _)| id);
    for (id, val) in &results_vec {
        println!("  thread {id}: computed {val}");
    }

    // --- Step 21: allow_threads pattern ---
    println!("\n  Step 21 — allow_threads (release GIL for CPU work):");
    Python::with_gil(|py| {
        println!("  GIL acquired — can access Python objects");
        let result = py.allow_threads(|| {
            let data: Vec<f64> = (1..=100_000).map(|x| x as f64).collect();
            let sum: f64 = data.iter().map(|x| x * x).sum();
            sum.sqrt()
        });
        println!("  GIL re-acquired — result: {result:.2}");
    });

    // --- Step 22: Py<T> vs Bound<'py, T> ---
    println!("\n  Step 22 — Py<T> vs Bound<'py, T>:");
    println!("  Bound<'py, T>: tied to GIL lifetime, cannot outlive with_gil");
    println!("  Py<T>: owned, Send, can be stored in structs / sent to threads");
    println!("  Use .bind(py) to convert Py<T> → &Bound<'py, T>");

    // --- Step 23: GIL rules summary ---
    println!("\n  Step 23 — GIL rules:");
    println!("  - Only one thread executes Python bytecode at a time");
    println!("  - Release GIL (allow_threads) for CPU work not touching Python");
    println!("  - Python 3.13 introduced experimental free-threaded build; 3.14+ supports it officially");
    println!("  - #[pyclass] requires Send (objects can be dropped from any thread)");
}

// ===========================================================================
// Section 6: Maturin & Toolchain
// ===========================================================================

fn maturin_and_toolchain() {
    println!("\n=== Section 6: Maturin & Toolchain ===\n");

    println!("  Maturin Command Reference:");
    println!("  ─────────────────────────────────────────────────────");
    println!("  maturin new <name>         Create a new PyO3 project");
    println!("  maturin develop            Build + install into virtualenv");
    println!("  maturin develop --release  Same, with optimizations");
    println!("  maturin build              Build a wheel (.whl)");
    println!("  maturin build --release    Build an optimized wheel");
    println!("  maturin publish            Build + upload to PyPI");
    println!();
    println!("  Project Structure:");
    println!("  my-project/");
    println!("  ├── Cargo.toml          [lib] crate-type = [\"cdylib\"]");
    println!("  ├── pyproject.toml      build-system requires maturin");
    println!("  ├── src/");
    println!("  │   └── lib.rs          #[pymodule] entry point");
    println!("  └── python/");
    println!("      └── my_module/");
    println!("          ├── __init__.py  re-export Rust + pure Python");
    println!("          └── utils.py    pure Python code");
    println!();
    println!("  Cargo.toml:");
    println!("    [lib]");
    println!("    name = \"my_module\"");
    println!("    crate-type = [\"cdylib\"]");
    println!("    [dependencies]");
    println!("    pyo3 = {{ version = \"0.23\", features = [\"extension-module\"] }}");
    println!();
    println!("  extension-module feature: tells PyO3 not to link libpython");
    println!("  (on macOS/Linux, symbols come from the interpreter itself)");
}

// ===========================================================================
// Section 7: Async Interop
// ===========================================================================

async fn async_computation(data: Vec<f64>) -> f64 {
    let sum: f64 = data.iter().map(|x| x * x).sum();
    sum.sqrt()
}

fn async_interop() {
    println!("\n=== Section 7: Async Interop ===\n");

    let rt = tokio::runtime::Runtime::new().unwrap();

    // --- Step 24: tokio Runtime + block_on ---
    println!("  Step 24 — future_into_py pattern (Rust async → Python awaitable):");
    let result = rt.block_on(async {
        async_computation(vec![1.0, 2.0, 3.0, 4.0, 5.0]).await
    });
    println!("  async_computation result: {result:.4}");

    // --- Step 25: Concurrent tasks with tokio::spawn ---
    println!("\n  Step 25 — Concurrent Rust futures:");
    let results: Vec<f64> = rt.block_on(async {
        let tasks: Vec<_> = (1..=4)
            .map(|i| {
                tokio::spawn(async move {
                    let values: Vec<f64> = (1..=1000).map(|x| (x * i) as f64).collect();
                    let sum: f64 = values.iter().sum();
                    sum / values.len() as f64
                })
            })
            .collect();

        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await.unwrap());
        }
        results
    });
    for (i, result) in results.iter().enumerate() {
        println!("  task {}: average = {result:.1}", i + 1);
    }

    // --- Step 26: Simulated Python coroutine call ---
    println!("\n  Step 26 — into_future pattern (Python awaitable → Rust future):");
    let result = rt.block_on(async {
        let simulated_python_coroutine = async { 42_i64 };
        simulated_python_coroutine.await
    });
    println!("  Python coroutine result: {result}");

    // --- Step 27: GIL + async key points ---
    println!("\n  Step 27 — GIL + async key points:");
    println!("  - Rust futures in future_into_py run WITHOUT the GIL");
    println!("  - GIL acquired only when returning results to Python");
    println!("  - No need for allow_threads inside future_into_py");
    println!("  - Use pyo3-async-runtimes crate for the bridge");
}

// ===========================================================================
// Section 8: cffi/ctypes vs PyO3
// ===========================================================================

// C-ABI style functions for ctypes/cffi
#[unsafe(no_mangle)]
pub extern "C" fn tut_ffi_add(a: i32, b: i32) -> i32 {
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn tut_ffi_magnitude(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

fn cffi_ctypes_vs_pyo3() {
    println!("\n=== Section 8: cffi/ctypes vs PyO3 ===\n");

    // --- Step 28: C-ABI functions ---
    println!("  Step 28 — C-ABI functions (extern \"C\" for ctypes/cffi):");
    println!("  ffi_add(2, 3) = {}", tut_ffi_add(2, 3));
    println!("  ffi_magnitude(3.0, 4.0) = {}", tut_ffi_magnitude(3.0, 4.0));

    // --- Step 29: Feature comparison ---
    println!("\n  Step 29 — Feature Comparison:");
    println!(
        "  {:<18} {:<14} {:<14} {:<16}",
        "Feature", "ctypes", "cffi", "PyO3"
    );
    println!("  {:-<18} {:-<14} {:-<14} {:-<16}", "", "", "", "");
    println!(
        "  {:<18} {:<14} {:<14} {:<16}",
        "Dependency", "None (stdlib)", "cffi pkg", "None (ext)"
    );
    println!(
        "  {:<18} {:<14} {:<14} {:<16}",
        "Type safety", "Manual", "C decls", "Automatic"
    );
    println!(
        "  {:<18} {:<14} {:<14} {:<16}",
        "Classes", "Difficult", "Difficult", "#[pyclass]"
    );
    println!(
        "  {:<18} {:<14} {:<14} {:<16}",
        "Async support", "No", "No", "Yes"
    );
    println!(
        "  {:<18} {:<14} {:<14} {:<16}",
        "Error handling", "Error codes", "Error codes", "Exceptions"
    );
    println!(
        "  {:<18} {:<14} {:<14} {:<16}",
        "Distribution", "Ship .so", "Ship .so", "pip install"
    );

    // --- Step 30: Python calling code ---
    println!("\n  Step 30 — Python calling code for each approach:");
    println!();
    println!("  # ctypes (stdlib)");
    println!("  import ctypes");
    println!("  lib = ctypes.CDLL('./libmy_lib.so')");
    println!("  lib.ffi_add.argtypes = [ctypes.c_int, ctypes.c_int]");
    println!("  lib.ffi_add.restype = ctypes.c_int");
    println!("  result = lib.ffi_add(2, 3)");
    println!();
    println!("  # cffi");
    println!("  from cffi import FFI");
    println!("  ffi = FFI()");
    println!("  ffi.cdef('int ffi_add(int a, int b);')");
    println!("  lib = ffi.dlopen('./libmy_lib.so')");
    println!("  result = lib.ffi_add(2, 3)");
    println!();
    println!("  # PyO3");
    println!("  import my_module");
    println!("  result = my_module.add(2, 3)  # just works!");

    // --- Step 31: When to use each ---
    println!("\n  Step 31 — When to use each:");
    println!("  PyO3    : new projects, need classes/async, Python-native feel");
    println!("  cffi    : existing C-ABI lib, multi-language support needed");
    println!("  ctypes  : quick scripts, stdlib-only, avoid for production");
}

// ===========================================================================
// Section 9: Best Practices
// ===========================================================================

fn best_practices() {
    println!("\n=== Section 9: Best Practices ===\n");

    println!("  Performance:");
    println!("  - Minimize GIL hold time — release with allow_threads for CPU work");
    println!("  - Convert types at boundaries, use native Rust types internally");
    println!("  - Batch operations: pass Vec<T> instead of calling per-element\n");

    println!("  Error handling:");
    println!("  - Return PyResult<T> from all #[pyfunction]s");
    println!("  - Map Rust errors to appropriate Python exceptions via From");
    println!("  - PyO3 catches panics as PanicException (but avoid panicking)\n");

    println!("  Building and distribution:");
    println!("  - Use maturin for building (maturin develop for iteration)");
    println!("  - crate-type = [\"cdylib\"] + extension-module feature");
    println!("  - pip install for end users (maturin publish for PyPI)\n");

    println!("  Testing:");
    println!("  - Rust unit tests: cargo test (test core logic without Python)");
    println!("  - Python integration tests: pytest (test the Python interface)");
    println!("  - Keep complex logic in Rust, thin wrappers in #[pyfunction]\n");

    println!("  Design:");
    println!("  - #[pyclass] requires Send — use Arc/Mutex, not Rc/Cell");
    println!("  - Fields must be owned (no &'a T in #[pyclass])");
    println!("  - Use Py<T> for storing Python objects outside the GIL");
    println!("  - Prefer Bound<'py, T> over Py<T> when inside with_gil");
}

// ===========================================================================
// Public entry point
// ===========================================================================

pub fn run() {
    why_python_rust();
    pyo3_fundamentals();
    type_conversions();
    exposing_structs();
    error_handling();
    the_gil();
    maturin_and_toolchain();
    async_interop();
    cffi_ctypes_vs_pyo3();
    best_practices();
}
