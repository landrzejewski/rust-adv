use std::fmt;
use std::ops::{BitAnd, BitOr, Not};

// ============================
// bitflags! macro
// ============================

macro_rules! bitflags {
    ($name:ident : $repr:ty { $($flag:ident = $val:expr),* $(,)? }) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        struct $name($repr);

        // Associated constants
        impl $name {
            $(const $flag: $name = $name($val);)*

            const fn empty() -> Self {
                $name(0)
            }

            fn all() -> Self {
                $name(0 $(| $val)*)
            }

            fn contains(self, other: Self) -> bool {
                (self.0 & other.0) == other.0
            }

            fn insert(&mut self, other: Self) {
                self.0 |= other.0;
            }

            fn remove(&mut self, other: Self) {
                self.0 &= !other.0;
            }

            fn toggle(&mut self, other: Self) {
                self.0 ^= other.0;
            }

            fn is_empty(self) -> bool {
                self.0 == 0
            }

            fn bits(self) -> $repr {
                self.0
            }
        }

        impl BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self {
                $name(self.0 | rhs.0)
            }
        }

        impl BitAnd for $name {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self {
                $name(self.0 & rhs.0)
            }
        }

        impl Not for $name {
            type Output = Self;
            fn not(self) -> Self {
                $name(!self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({:#0width$b})", stringify!($name), self.0,
                    width = (std::mem::size_of::<$repr>() * 8 + 2))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.is_empty() {
                    return write!(f, "(empty)");
                }
                let mut first = true;
                $(
                    if self.contains($name($val)) {
                        if !first {
                            write!(f, " | ")?;
                        }
                        write!(f, "{}", stringify!($flag))?;
                        first = false;
                    }
                )*
                if first {
                    // Bits set that don't match any named flag
                    write!(f, "({:#0width$b})", self.0,
                        width = (std::mem::size_of::<$repr>() * 8 + 2))?;
                }
                Ok(())
            }
        }
    };
}

// ============================
// Define flags types
// ============================

bitflags!(Permissions: u8 {
    READ    = 0b0001,
    WRITE   = 0b0010,
    EXECUTE = 0b0100,
    DELETE  = 0b1000,
});

bitflags!(FileMode: u16 {
    OWNER_READ    = 0o400,
    OWNER_WRITE   = 0o200,
    OWNER_EXEC    = 0o100,
    GROUP_READ    = 0o040,
    GROUP_WRITE   = 0o020,
    GROUP_EXEC    = 0o010,
    OTHER_READ    = 0o004,
    OTHER_WRITE   = 0o002,
    OTHER_EXEC    = 0o001,
});

// ============================
// Demonstration
// ============================

pub fn run() {
    println!("=== Exercise 13: Bitflags Macro ===\n");

    // --- Basic flag operations ---
    println!("--- Permissions (u8 backing) ---");
    let rw = Permissions::READ | Permissions::WRITE;
    println!("  READ | WRITE = {rw}");
    println!("  Debug: {rw:?}");
    println!("  Contains READ?    {}", rw.contains(Permissions::READ));
    println!("  Contains EXECUTE? {}", rw.contains(Permissions::EXECUTE));

    // --- Insert / Remove / Toggle ---
    println!("\n--- Mutating flags ---");
    let mut perms = Permissions::READ;
    println!("  Start:          {perms}");
    perms.insert(Permissions::WRITE | Permissions::EXECUTE);
    println!("  After insert:   {perms}");
    perms.remove(Permissions::WRITE);
    println!("  After remove W: {perms}");
    perms.toggle(Permissions::DELETE);
    println!("  After toggle D: {perms}");
    perms.toggle(Permissions::DELETE);
    println!("  Toggle D again: {perms}");

    // --- all() and empty() ---
    println!("\n--- Special values ---");
    let all = Permissions::all();
    let empty = Permissions::empty();
    println!("  all()   = {all} (bits: {:#010b})", all.bits());
    println!("  empty() = {empty}");
    println!("  empty is_empty: {}", empty.is_empty());

    // --- BitAnd / Not ---
    println!("\n--- BitAnd / Not ---");
    let rwe = Permissions::READ | Permissions::WRITE | Permissions::EXECUTE;
    let re = Permissions::READ | Permissions::EXECUTE;
    let common = rwe & re;
    println!("  (R|W|X) & (R|X) = {common}");
    let inverted = !Permissions::READ;
    println!("  !READ = {:?}", inverted);

    // --- Second flags type (FileMode: u16) ---
    println!("\n--- FileMode (u16 backing) ---");
    let mode_644 =
        FileMode::OWNER_READ | FileMode::OWNER_WRITE | FileMode::GROUP_READ | FileMode::OTHER_READ;
    println!("  644 = {mode_644}");
    println!("  Debug: {mode_644:?}");
    println!("  Contains OWNER_WRITE? {}", mode_644.contains(FileMode::OWNER_WRITE));
    println!("  Contains GROUP_WRITE? {}", mode_644.contains(FileMode::GROUP_WRITE));

    let mode_755 = FileMode::OWNER_READ
        | FileMode::OWNER_WRITE
        | FileMode::OWNER_EXEC
        | FileMode::GROUP_READ
        | FileMode::GROUP_EXEC
        | FileMode::OTHER_READ
        | FileMode::OTHER_EXEC;
    println!("  755 = {mode_755}");

    let all_modes = FileMode::all();
    println!("  all() = {all_modes}");
    println!("  all() bits: {:#018b}", all_modes.bits());

    // --- Equality ---
    println!("\n--- Equality ---");
    let a = Permissions::READ | Permissions::WRITE;
    let b = Permissions::WRITE | Permissions::READ;
    println!("  R|W == W|R? {}", a == b);
    println!("  R|W == R?   {}", a == Permissions::READ);
}
