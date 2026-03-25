use std::panic;

// ============================
// test_suite! macro
// ============================

macro_rules! test_suite {
    ($suite:ident { $($name:expr => $body:block),* $(,)? }) => {
        struct $suite {
            results: Vec<(&'static str, bool)>,
        }

        impl $suite {
            fn run() -> Self {
                let mut results = Vec::new();
                $(
                    test_suite!(@run_single results, $name, $body);
                )*
                $suite { results }
            }

            fn summary(&self) {
                println!("  Suite: {}", stringify!($suite));
                let mut passed = 0;
                let mut failed = 0;
                for (name, ok) in &self.results {
                    if *ok {
                        println!("    [PASS] {name}");
                        passed += 1;
                    } else {
                        println!("    [FAIL] {name}");
                        failed += 1;
                    }
                }
                let total = passed + failed;
                println!("  Result: {passed}/{total} passed, {failed} failed");
            }

            fn all_passed(&self) -> bool {
                self.results.iter().all(|(_, ok)| *ok)
            }

            fn results(&self) -> &[(&'static str, bool)] {
                &self.results
            }
        }
    };

    // Internal rule: run a single test with catch_unwind
    (@run_single $results:ident, $name:expr, $body:block) => {
        let outcome = panic::catch_unwind(panic::AssertUnwindSafe(|| $body));
        $results.push(($name, outcome.is_ok()));
    };
}

// ============================
// test_group! macro — combines multiple suites
// ============================

macro_rules! test_group {
    (run_all: $($suite:ident),* $(,)?) => {{
        let mut all_results: Vec<(&'static str, bool)> = Vec::new();
        $(
            let suite = $suite::run();
            suite.summary();
            all_results.extend_from_slice(suite.results());
            println!();
        )*

        let passed = all_results.iter().filter(|(_, ok)| *ok).count();
        let total = all_results.len();
        let failed = total - passed;
        println!("  === Group Total: {passed}/{total} passed, {failed} failed ===");
        all_results.iter().all(|(_, ok)| *ok)
    }};
}

// ============================
// Define test suites
// ============================

test_suite!(MathTests {
    "addition" => {
        assert_eq!(2 + 2, 4);
    },
    "subtraction" => {
        assert_eq!(10 - 3, 7);
    },
    "division" => {
        assert_eq!(10 / 2, 5);
    },
    "multiplication overflow check" => {
        assert_eq!(100 * 100, 10_000);
    },
    "deliberate failure" => {
        assert_eq!(1 + 1, 3, "This should fail");
    },
});

test_suite!(StringTests {
    "concatenation" => {
        let s = format!("{}{}", "hello", " world");
        assert_eq!(s, "hello world");
    },
    "length" => {
        assert_eq!("rust".len(), 4);
    },
    "uppercase" => {
        assert_eq!("hello".to_uppercase(), "HELLO");
    },
    "contains" => {
        assert!("hello world".contains("world"));
    },
    "empty check" => {
        assert!(!"".is_empty() || true); // passes
    },
});

test_suite!(EdgeCaseTests {
    "panic in test" => {
        panic!("intentional panic");
    },
    "index out of bounds" => {
        let v = vec![1, 2, 3];
        let _ = v[10]; // panics
    },
    "normal test after panics" => {
        assert_eq!(42, 42);
    },
});

// ============================
// Demonstration
// ============================

pub fn run() {
    println!("=== Exercise 12: Test Suite Runner Macro ===\n");

    // --- Run individual suite ---
    println!("--- Individual suite ---");
    let math = MathTests::run();
    math.summary();
    println!("  All passed? {}", math.all_passed());

    // --- Run another suite ---
    println!("\n--- String tests ---");
    let strings = StringTests::run();
    strings.summary();
    println!("  All passed? {}", strings.all_passed());

    // --- Suite with panics caught by catch_unwind ---
    println!("\n--- Edge cases (panics caught) ---");
    let edges = EdgeCaseTests::run();
    edges.summary();
    println!("  All passed? {}", edges.all_passed());

    // --- Combined group ---
    println!("\n--- Test group (all suites) ---");
    let all_ok = test_group!(run_all: MathTests, StringTests, EdgeCaseTests);
    println!("  Group all passed? {all_ok}");
}
