// ============================
// make_builder! macro
// ============================

macro_rules! make_builder {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        // Generate the builder struct
        paste_builder_name!($name { $($field : $ty),* });
    };
}

// Helper macro to construct the builder name by appending "Builder"
// (Since declarative macros can't concatenate identifiers, we define
// the builder with a known naming pattern using a nested macro.)
macro_rules! paste_builder_name {
    ($name:ident { $($field:ident : $ty:ty),* }) => {
        // The builder struct with all fields as Option<T>
        #[derive(Default)]
        struct Builder {
            $($field: Option<$ty>,)*
        }

        impl Builder {
            fn new() -> Self {
                Self::default()
            }

            // Chainable setter for each field
            $(
                fn $field(&mut self, value: $ty) -> &mut Self {
                    self.$field = Some(value);
                    self
                }
            )*

            // Build method: returns Ok if all fields are set
            fn build(&mut self) -> Result<$name, String> {
                Ok($name {
                    $(
                        $field: self.$field.take().ok_or_else(||
                            format!("Missing field: '{}'", stringify!($field))
                        )?,
                    )*
                })
            }
        }
    };
}

// ============================
// Define a struct and its builder
// ============================

#[derive(Debug)]
struct Person {
    name: String,
    age: u32,
    email: String,
}

make_builder!(Person {
    name: String,
    age: u32,
    email: String,
});

// ============================
// Demonstration
// ============================

pub fn run() {
    println!("=== Exercise 14: Builder Macro ===\n");

    // --- Successful build ---
    println!("--- Successful build ---");
    let person = Builder::new()
        .name("Alice".to_string())
        .age(30)
        .email("alice@example.com".to_string())
        .build();

    match person {
        Ok(p) => println!("  Built: {:?}", p),
        Err(e) => println!("  Error: {}", e),
    }

    // --- Chaining in different order ---
    println!("\n--- Different field order ---");
    let person = Builder::new()
        .email("bob@company.org".to_string())
        .name("Bob".to_string())
        .age(25)
        .build();

    match person {
        Ok(p) => println!("  Built: {:?}", p),
        Err(e) => println!("  Error: {}", e),
    }

    // --- Missing field: should report error ---
    println!("\n--- Missing field ---");
    let result = Builder::new()
        .name("Charlie".to_string())
        .build();

    match result {
        Ok(p) => println!("  Built: {:?}", p),
        Err(e) => println!("  Error: {}", e),
    }

    // --- Multiple missing fields ---
    println!("\n--- Only age set ---");
    let result = Builder::new().age(99).build();

    match result {
        Ok(p) => println!("  Built: {:?}", p),
        Err(e) => println!("  Error: {}", e),
    }

    // --- Empty builder ---
    println!("\n--- Empty builder ---");
    let result = Builder::new().build();

    match result {
        Ok(p) => println!("  Built: {:?}", p),
        Err(e) => println!("  Error: {}", e),
    }
}
