use std::fmt;
use std::marker::PhantomData;

// ============================
// Permission type with const generic booleans
// ============================

#[derive(Debug)]
struct Permission<const R: bool, const W: bool, const X: bool>;

// Common permission aliases
type ReadOnly = Permission<true, false, false>;
type ReadWrite = Permission<true, true, false>;
type ReadExecute = Permission<true, false, true>;
type FullAccess = Permission<true, true, true>;
type NoAccess = Permission<false, false, false>;

impl<const R: bool, const W: bool, const X: bool> fmt::Display for Permission<R, W, X> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            if R { 'r' } else { '-' },
            if W { 'w' } else { '-' },
            if X { 'x' } else { '-' },
        )
    }
}

// ============================
// FileHandle parameterized by permission
// ============================

struct FileHandle<P> {
    path: String,
    content: String,
    _perm: PhantomData<P>,
}

impl<P> FileHandle<P> {
    fn path(&self) -> &str {
        &self.path
    }
}

// Conditional: read available only when R = true
impl<const W: bool, const X: bool> FileHandle<Permission<true, W, X>> {
    fn read(&self) -> &str {
        &self.content
    }
}

// Conditional: write available only when W = true
impl<const R: bool, const X: bool> FileHandle<Permission<R, true, X>> {
    fn write(&mut self, data: &str) {
        self.content.push_str(data);
    }
}

// Conditional: execute available only when X = true
impl<const R: bool, const W: bool> FileHandle<Permission<R, W, true>> {
    fn execute(&self) -> i32 {
        // Simulate execution: return "exit code" based on content length
        (self.content.len() % 256) as i32
    }
}

// Constructor: open with specific permission
fn open_file<P>(path: &str, content: &str) -> FileHandle<P> {
    FileHandle {
        path: path.to_string(),
        content: content.to_string(),
        _perm: PhantomData,
    }
}

// ============================
// Newtypes: UserId / GroupId
// ============================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UserId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GroupId(u32);

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "uid:{}", self.0)
    }
}

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gid:{}", self.0)
    }
}

fn file_info(path: &str, owner: UserId, group: GroupId) -> String {
    format!("{} owner={} group={}", path, owner, group)
}

// ============================
// Extension trait: parse permission strings
// ============================

trait PermissionDescExt {
    fn parse_permissions(&self) -> Result<(bool, bool, bool), String>;
}

impl PermissionDescExt for &str {
    fn parse_permissions(&self) -> Result<(bool, bool, bool), String> {
        let chars: Vec<char> = self.chars().collect();
        if chars.len() != 3 {
            return Err(format!("Expected 3 characters, got {}", chars.len()));
        }
        let r = match chars[0] {
            'r' => true,
            '-' => false,
            c => return Err(format!("Invalid read char: '{c}'")),
        };
        let w = match chars[1] {
            'w' => true,
            '-' => false,
            c => return Err(format!("Invalid write char: '{c}'")),
        };
        let x = match chars[2] {
            'x' => true,
            '-' => false,
            c => return Err(format!("Invalid execute char: '{c}'")),
        };
        Ok((r, w, x))
    }
}

// ============================
// Marker trait: Auditable (only for readable handles)
// ============================

trait Auditable {
    fn audit_log(&self) -> String;
}

// Blanket impl: any FileHandle that is readable is auditable
impl<const W: bool, const X: bool> Auditable for FileHandle<Permission<true, W, X>> {
    fn audit_log(&self) -> String {
        format!("[AUDIT] Read access on '{}'", self.path)
    }
}

// ============================
// Demonstration
// ============================

fn demo_auditable(handle: &dyn Auditable) {
    println!("  {}", handle.audit_log());
}

pub fn run() {
    println!("=== Exercise 9: Unix Permission System ===\n");

    // --- Permission display ---
    println!("--- Permission combinations ---");
    println!("  ReadOnly:    {}", Permission::<true, false, false>);
    println!("  ReadWrite:   {}", Permission::<true, true, false>);
    println!("  ReadExecute: {}", Permission::<true, false, true>);
    println!("  FullAccess:  {}", Permission::<true, true, true>);
    println!("  NoAccess:    {}", Permission::<false, false, false>);

    // --- FileHandle with ReadWrite ---
    println!("\n--- ReadWrite file handle ---");
    let mut rw_file: FileHandle<ReadWrite> = open_file("/tmp/data.txt", "Hello");
    println!("  Path: {}", rw_file.path());
    println!("  Read: '{}'", rw_file.read());
    rw_file.write(", World!");
    println!("  After write: '{}'", rw_file.read());
    // rw_file.execute(); // Compile error! No X permission

    // --- FileHandle with ReadExecute ---
    println!("\n--- ReadExecute file handle ---");
    let rx_file: FileHandle<ReadExecute> = open_file("/usr/bin/tool", "#!/bin/sh\necho hi");
    println!("  Read: '{}'", rx_file.read());
    println!("  Execute exit code: {}", rx_file.execute());
    // rx_file.write("x"); // Compile error! No W permission

    // --- FileHandle with FullAccess ---
    println!("\n--- FullAccess file handle ---");
    let mut full_file: FileHandle<FullAccess> = open_file("/home/user/script.sh", "data");
    println!("  Read: '{}'", full_file.read());
    full_file.write("_appended");
    println!("  After write: '{}'", full_file.read());
    println!("  Execute exit code: {}", full_file.execute());

    // --- NoAccess: no read/write/execute methods available ---
    println!("\n--- NoAccess file handle ---");
    let no_file: FileHandle<NoAccess> = open_file("/secret/locked", "classified");
    println!("  Path: {} (no operations available)", no_file.path());
    // no_file.read();    // Compile error!
    // no_file.write(""); // Compile error!
    // no_file.execute(); // Compile error!

    // --- Newtypes prevent argument swapping ---
    println!("\n--- Newtypes: UserId / GroupId ---");
    let uid = UserId(1000);
    let gid = GroupId(100);
    println!("  {}", file_info("/etc/config", uid, gid));
    // file_info("/etc/config", gid, uid); // Compile error! Types don't match

    // --- Extension trait: parse permission strings ---
    println!("\n--- Permission string parsing ---");
    for desc in ["rwx", "r-x", "rw-", "r--", "---"] {
        match desc.parse_permissions() {
            Ok((r, w, x)) => println!("  \"{desc}\" => read={r}, write={w}, execute={x}"),
            Err(e) => println!("  \"{desc}\" => Error: {e}"),
        }
    }
    println!("  Invalid:");
    for bad in ["rx", "abc", "rrrr"] {
        match bad.parse_permissions() {
            Ok(p) => println!("  \"{bad}\" => {:?}", p),
            Err(e) => println!("  \"{bad}\" => Error: {e}"),
        }
    }

    // --- Auditable marker trait ---
    println!("\n--- Auditable trait (readable handles only) ---");
    let ro_file: FileHandle<ReadOnly> = open_file("/var/log/app.log", "log data");
    demo_auditable(&ro_file);
    demo_auditable(&rw_file);
    demo_auditable(&rx_file);
    demo_auditable(&full_file);
    // demo_auditable(&no_file); // Compile error! NoAccess doesn't impl Auditable
}
