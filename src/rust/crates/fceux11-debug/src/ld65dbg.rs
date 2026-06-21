//! ld65 `.dbg` file parser — replacement for C++ `src/ld65dbg.{h,cpp}` (558 LoC).
//!
//! Parses cc65/ld65 debug output (e.g. `MyProgram.dbg`) and exposes the
//! contained symbols as `FceuLd65Sym` POD records via FFI. The C++ side
//! (`debugsymboltable.cpp::ld65LoadDebugFile`) opens via [`ld65_open`],
//! iterates via [`ld65_iterate`] or pull-style [`ld65_sym_count`] +
//! [`ld65_sym_get`], then closes via [`ld65_close`].
//!
//! # Recognised records
//!
//! Only three record kinds are processed (matches existing C++ behaviour):
//!
//! * `seg`  — segments (`id=`, `name=`, `ooffs=`)
//! * `scope` — scopes  (`id=`, `name=`, `size=`, `parent=`)
//! * `sym`  — symbols  (`id=`, `name=`, `size=`, `val=`, `scope=`, `seg=`, `type=`)
//!
//! All other records (`version`, `info`, `mod`, `file`, `line`, `span`, `lib`,
//! `csym`, etc.) are silently skipped.
//!
//! # Format
//!
//! Each record is one line: `keyword<TAB>key=val,key=val,...<LF>`. String
//! values are wrapped in `"…"` (no escapes — cc65 does not emit them).

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Mutex;

// --------------------------------------------------------------------------
// Internal data
// --------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymType {
    Import = 0,
    Label = 1,
    Equ = 2,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub id: u32,
    pub name: String,
    pub size: i32,
    pub parent_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub id: u32,
    pub name: String,
    pub ofs: i32,
}

#[derive(Debug, Clone)]
pub struct Sym {
    pub id: u32,
    pub name: String,
    pub size: i32,
    pub value: i32,
    pub sym_type: SymType,
    pub scope_id: Option<u32>,
    pub segment_id: Option<u32>,
}

/// Parsed `.dbg` content. Owned by a single [`ld65_open`] handle.
pub struct Database {
    pub scopes: HashMap<u32, Scope>,
    pub segments: HashMap<u32, Segment>,
    /// Insertion-ordered list — matches the C++ `std::map<int, sym*>` id-sorted
    /// iteration order because cc65 writes records in monotonically-increasing
    /// id order.
    pub syms: Vec<Sym>,
    /// Cached `name_cstring` for each sym, in the same order. The pointer
    /// handed out via FFI is borrowed from this Vec.
    pub sym_name_cstrings: Vec<CString>,
    /// Cached segment name CStrings keyed by segment id.
    pub seg_name_cstrings: HashMap<u32, CString>,
    /// Cached full-scope-path CStrings keyed by scope id (computed
    /// iteratively via [`Scope::full_name`]).
    pub scope_full_name_cstrings: HashMap<u32, CString>,
}

impl Database {
    fn new() -> Self {
        Database {
            scopes: HashMap::new(),
            segments: HashMap::new(),
            syms: Vec::new(),
            sym_name_cstrings: Vec::new(),
            seg_name_cstrings: HashMap::new(),
            scope_full_name_cstrings: HashMap::new(),
        }
    }

    /// Build full dotted name for `scope_id` by walking parents iteratively.
    /// Matches the C++ `scope::getFullName` behaviour: each non-empty name is
    /// appended followed by `"::"`. Returns `"A::B::C::"` etc. (trailing
    /// `"::"` preserved for backward C++ compat).
    pub fn build_full_scope_name(&self, scope_id: u32) -> String {
        // Walk up the parent chain, collecting names from root → leaf.
        let mut chain: Vec<&str> = Vec::new();
        let mut cur = self.scopes.get(&scope_id);
        // Safety guard against accidental cycles in malformed files (~32 max).
        let mut guard = 0;
        while let Some(s) = cur {
            guard += 1;
            if guard > 64 {
                break;
            }
            // Skip empty names — matches C++ behaviour: `if (!_name.empty())`.
            if !s.name.is_empty() {
                chain.push(&s.name);
            }
            cur = match s.parent_id {
                Some(pid) => self.scopes.get(&pid),
                None => None,
            };
        }
        // C++ traverses parent → self (recursive call before the append), so
        // root-most name comes first. Our `chain` is leaf → root, so reverse.
        chain.reverse();
        let mut out = String::new();
        for n in chain {
            out.push_str(n);
            out.push_str("::");
        }
        out
    }

    fn build_caches(&mut self) {
        self.sym_name_cstrings = self
            .syms
            .iter()
            .map(|s| CString::new(s.name.as_str()).unwrap_or_default())
            .collect();
        self.seg_name_cstrings = self
            .segments
            .iter()
            .map(|(id, s)| (*id, CString::new(s.name.as_str()).unwrap_or_default()))
            .collect();
        let scope_ids: Vec<u32> = self.scopes.keys().copied().collect();
        for sid in scope_ids {
            let full = self.build_full_scope_name(sid);
            self.scope_full_name_cstrings
                .insert(sid, CString::new(full).unwrap_or_default());
        }
    }
}

// --------------------------------------------------------------------------
// Parse error
// --------------------------------------------------------------------------

#[derive(Debug)]
pub enum ParseError {
    Io(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Io(s) => write!(f, "I/O error: {}", s),
        }
    }
}

// --------------------------------------------------------------------------
// Tokenisation
// --------------------------------------------------------------------------

/// Split `body` (the post-keyword portion of a record line) into key=value
/// pairs. Handles `"…"` strings (commas inside quotes don't split), strips
/// whitespace outside quotes, and removes the quote characters themselves
/// (matching the C++ `readKeyValuePair` behaviour exactly).
fn parse_kv_pairs(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip leading whitespace.
        while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        // Read key (alphanumeric / underscore).
        let key_start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        if i == key_start {
            // Unrecognised junk — skip to next separator.
            while i < bytes.len() && bytes[i] != b',' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        let key = std::str::from_utf8(&bytes[key_start..i])
            .unwrap_or("")
            .to_string();

        // Skip whitespace, then expect '='.
        while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            // Key with no value — skip to next separator.
            while i < bytes.len() && bytes[i] != b',' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        i += 1; // consume '='
        while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }

        // Read value — stop at unquoted ','. Strip quotes; ignore whitespace
        // outside quotes (matches C++).
        let mut value = String::new();
        let mut in_string = false;
        while i < bytes.len() {
            let c = bytes[i];
            if !in_string && c == b',' {
                break;
            } else if c == b'"' {
                in_string = !in_string;
                i += 1;
            } else {
                if !(c as char).is_ascii_whitespace() {
                    value.push(c as char);
                }
                i += 1;
            }
        }
        if i < bytes.len() && bytes[i] == b',' {
            i += 1;
        }
        out.push((key, value));
    }
    out
}

/// Parse an integer that may be prefixed with `0x` (matches C++ `strtol(...,0)`).
fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(rest, 16).ok()
    } else {
        s.parse::<i64>().ok()
    }
}

// --------------------------------------------------------------------------
// Load
// --------------------------------------------------------------------------

/// Read and parse a `.dbg` file. Returns a fully-built [`Database`] (with
/// CStrings cached for FFI use) or an [`ParseError`] if I/O fails. Malformed
/// lines are silently dropped — matches the C++ behaviour.
pub fn load_from_path(path: &str) -> Result<Database, ParseError> {
    let f = File::open(path).map_err(|e| ParseError::Io(e.to_string()))?;
    let reader = BufReader::new(f);
    let mut db = Database::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }

        // First token (record kind) — terminated by TAB or whitespace.
        let kind_end = trimmed
            .bytes()
            .position(|b| b == b'\t' || b == b' ')
            .unwrap_or(trimmed.len());
        let kind = &trimmed[..kind_end];
        let body = if kind_end < trimmed.len() {
            &trimmed[kind_end..]
        } else {
            ""
        };

        match kind {
            "seg" | "scope" | "sym" => {}
            _ => continue, // ignore version/info/mod/file/line/span/lib/csym/etc.
        }

        let kv = parse_kv_pairs(body);

        // Extract canonical fields.
        let mut id: Option<u32> = None;
        let mut name = String::new();
        let mut size: i32 = 0;
        let mut value: i32 = 0;
        let mut scope_id: Option<u32> = None;
        let mut parent_id: Option<u32> = None;
        let mut segment_id: Option<u32> = None;
        let mut ofs: i32 = -1;
        let mut type_str = String::new();

        for (k, v) in &kv {
            match k.as_str() {
                "id" => id = parse_int(v).and_then(|n| u32::try_from(n).ok()),
                "name" => name = v.clone(),
                "size" => size = parse_int(v).unwrap_or(0) as i32,
                "val" => value = parse_int(v).unwrap_or(0) as i32,
                "scope" => scope_id = parse_int(v).and_then(|n| u32::try_from(n).ok()),
                "parent" => parent_id = parse_int(v).and_then(|n| u32::try_from(n).ok()),
                "seg" => segment_id = parse_int(v).and_then(|n| u32::try_from(n).ok()),
                "ooffs" => ofs = parse_int(v).unwrap_or(-1) as i32,
                "type" => type_str = v.clone(),
                _ => {} // unknown key — ignored
            }
        }

        let Some(id) = id else { continue };

        match kind {
            "seg" => {
                db.segments.insert(
                    id,
                    Segment {
                        id,
                        name: name.clone(),
                        ofs,
                    },
                );
            }
            "scope" => {
                db.scopes.insert(
                    id,
                    Scope {
                        id,
                        name: name.clone(),
                        size,
                        parent_id,
                    },
                );
            }
            "sym" => {
                let sym_type = match type_str.as_str() {
                    "lab" => SymType::Label,
                    "equ" => SymType::Equ,
                    _ => SymType::Import,
                };
                db.syms.push(Sym {
                    id,
                    name: name.clone(),
                    size,
                    value,
                    sym_type,
                    scope_id,
                    segment_id,
                });
            }
            _ => unreachable!(),
        }
    }

    db.build_caches();
    Ok(db)
}

// --------------------------------------------------------------------------
// FFI surface — opaque Database handle + per-sym view struct
// --------------------------------------------------------------------------

/// Layout-compatible POD view of one `Sym` for C++ consumption. All pointer
/// fields borrow from the [`Database`] cache and remain valid until the
/// containing [`ld65_close`] call.
#[repr(C)]
pub struct FceuLd65Sym {
    pub id: u32,
    pub name_ptr: *const c_char,
    pub name_len: usize,
    pub size: i32,
    pub value: i32,
    /// `0` = IMPORT, `1` = LABEL, `2` = EQU.
    pub sym_type: i32,
    /// Non-zero iff `segment_*` fields are populated.
    pub has_segment: i32,
    pub segment_ofs: i32,
    pub segment_name_ptr: *const c_char,
    pub segment_name_len: usize,
    pub has_scope: i32,
    pub scope_full_name_ptr: *const c_char,
    pub scope_full_name_len: usize,
}

impl Default for FceuLd65Sym {
    fn default() -> Self {
        FceuLd65Sym {
            id: 0,
            name_ptr: std::ptr::null(),
            name_len: 0,
            size: 0,
            value: 0,
            sym_type: 0,
            has_segment: 0,
            segment_ofs: -1,
            segment_name_ptr: std::ptr::null(),
            segment_name_len: 0,
            has_scope: 0,
            scope_full_name_ptr: std::ptr::null(),
            scope_full_name_len: 0,
        }
    }
}

static LAST_ERROR: Mutex<Option<CString>> = Mutex::new(None);

fn set_last_error(msg: &str) {
    let mut e = LAST_ERROR.lock().unwrap_or_else(|p| p.into_inner());
    *e = Some(CString::new(msg).unwrap_or_default());
}

fn clear_last_error() {
    let mut e = LAST_ERROR.lock().unwrap_or_else(|p| p.into_inner());
    *e = None;
}

fn c_str_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

/// Open a `.dbg` file and parse it. Returns an opaque handle, or `NULL` on
/// error (call [`ld65_last_error`] for the reason).
///
/// # Safety
///
/// `path` must be a valid, null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ld65_open(path: *const c_char) -> *mut c_void {
    if path.is_null() {
        set_last_error("null path");
        return std::ptr::null_mut();
    }
    let p = c_str_to_string(path);
    match load_from_path(&p) {
        Ok(db) => {
            clear_last_error();
            Box::into_raw(Box::new(db)) as *mut c_void
        }
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Free a database returned by [`ld65_open`].
///
/// # Safety
///
/// `db` must be either null or a valid pointer returned by `ld65_open` that
/// has not already been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ld65_close(db: *mut c_void) {
    if !db.is_null() {
        unsafe { drop(Box::from_raw(db as *mut Database)) };
    }
}

/// Number of symbols in the database.
///
/// # Safety
///
/// `db` must be either null or a valid pointer returned by `ld65_open`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ld65_sym_count(db: *mut c_void) -> u32 {
    if db.is_null() {
        return 0;
    }
    let db = unsafe { &*(db as *const Database) };
    db.syms.len() as u32
}

/// Fill `out` with the symbol at index `idx`. Returns `true` on success.
///
/// # Safety
///
/// `db` must be either null or a valid pointer returned by `ld65_open`.
/// `out` must be a valid, writable pointer to a `FceuLd65Sym`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ld65_sym_get(
    db: *mut c_void,
    idx: u32,
    out: *mut FceuLd65Sym,
) -> bool {
    if db.is_null() || out.is_null() {
        return false;
    }
    let db_ref = unsafe { &*(db as *const Database) };
    let i = idx as usize;
    if i >= db_ref.syms.len() {
        return false;
    }
    let sym = &db_ref.syms[i];
    let name_cstr = &db_ref.sym_name_cstrings[i];

    let name_len = name_cstr.as_bytes().len();
    let mut view = FceuLd65Sym {
        id: sym.id,
        name_ptr: name_cstr.as_ptr(),
        name_len,
        size: sym.size,
        value: sym.value,
        sym_type: sym.sym_type.clone() as i32,
        ..Default::default()
    };

    if let Some(seg_id) = sym.segment_id
        && let Some(seg) = db_ref.segments.get(&seg_id)
        && let Some(seg_cstr) = db_ref.seg_name_cstrings.get(&seg_id)
    {
        view.has_segment = 1;
        view.segment_ofs = seg.ofs;
        view.segment_name_ptr = seg_cstr.as_ptr();
        view.segment_name_len = seg_cstr.as_bytes().len();
    }

    if let Some(scope_id) = sym.scope_id
        && let Some(scope_cstr) = db_ref.scope_full_name_cstrings.get(&scope_id)
    {
        view.has_scope = 1;
        view.scope_full_name_ptr = scope_cstr.as_ptr();
        view.scope_full_name_len = scope_cstr.as_bytes().len();
    }

    unsafe { std::ptr::write(out, view) };
    true
}

/// Callback-style iteration matching the legacy `database::iterateSymbols`
/// signature. `cb` is called once per symbol (in id order) with `user_data`
/// echoed back. Returns the number of symbols iterated.
///
/// # Safety
///
/// `db` must be either null or a valid pointer returned by `ld65_open`.
/// The callback `cb` must uphold the safety requirements for the `FceuLd65Sym`
/// pointer it receives (it is valid only for the duration of the call).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ld65_iterate(
    db: *mut c_void,
    user_data: *mut c_void,
    cb: Option<extern "C" fn(*mut c_void, *const FceuLd65Sym)>,
) -> u32 {
    let Some(callback) = cb else {
        return 0;
    };
    if db.is_null() {
        return 0;
    }
    let count = unsafe { fceux11_rust_ld65_sym_count(db) };
    for i in 0..count {
        let mut view = FceuLd65Sym::default();
        if unsafe { fceux11_rust_ld65_sym_get(db, i, &mut view) } {
            callback(user_data, &view);
        }
    }
    count
}

/// Return the most recent error message produced by [`ld65_open`], or `NULL`
/// if there isn't one.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_ld65_last_error() -> *const c_char {
    let e = LAST_ERROR.lock().unwrap_or_else(|p| p.into_inner());
    match &*e {
        Some(s) => s.as_ptr(),
        None => std::ptr::null(),
    }
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_dbg(content: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        // Use a counter for unique-enough names (no Math.random / Date.now needed
        // because the test framework spawns each test in its own process).
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        path.push(format!("fceux11_ld65_test_{}.dbg", n));
        let mut f = File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn parse_kv_basic() {
        unsafe {
            let r = parse_kv_pairs("\tid=5,name=\"foo\",val=0x1234");
            assert_eq!(
                r,
                vec![
                    ("id".to_string(), "5".to_string()),
                    ("name".to_string(), "foo".to_string()),
                    ("val".to_string(), "0x1234".to_string()),
                ]
            );
        }
    }

    #[test]
    fn parse_kv_string_with_comma_inside() {
        unsafe {
            // Quoted comma should NOT split the value.
            let r = parse_kv_pairs("\tname=\"a,b,c\",id=1");
            assert_eq!(r[0].0, "name");
            assert_eq!(r[0].1, "a,b,c"); // commas preserved inside quotes
            assert_eq!(r[1].0, "id");
        }
    }

    #[test]
    fn parse_int_handles_hex_and_decimal() {
        unsafe {
            assert_eq!(parse_int("0x10"), Some(16));
            assert_eq!(parse_int("42"), Some(42));
            assert_eq!(parse_int(""), None);
            assert_eq!(parse_int("garbage"), None);
        }
    }

    #[test]
    fn load_empty_file() {
        unsafe {
            let path = write_temp_dbg("");
            let db = load_from_path(path.to_str().unwrap()).unwrap();
            assert_eq!(db.syms.len(), 0);
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn load_version_info_only() {
        unsafe {
            let content = "version\tmajor=2,minor=0\ninfo\tcsym=0,file=0,lib=0\n";
            let path = write_temp_dbg(content);
            let db = load_from_path(path.to_str().unwrap()).unwrap();
            assert_eq!(db.syms.len(), 0);
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn load_full_record_set() {
        unsafe {
            let content = "\
version\tmajor=2,minor=0
info\tsym=1,seg=1,scope=1
seg\tid=0,name=\"CODE\",ooffs=16
scope\tid=0,name=\"global\",size=0
sym\tid=0,name=\"main\",size=0,val=0x8000,scope=0,seg=0,type=lab
sym\tid=1,name=\"reset\",val=0x8050,type=lab
";
            let path = write_temp_dbg(content);
            let db = load_from_path(path.to_str().unwrap()).unwrap();
            assert_eq!(db.syms.len(), 2);
            assert_eq!(db.syms[0].name, "main");
            assert_eq!(db.syms[0].value, 0x8000);
            assert_eq!(db.syms[0].sym_type, SymType::Label);
            assert_eq!(db.syms[1].name, "reset");
            assert_eq!(db.syms[1].value, 0x8050);
            assert_eq!(db.segments.len(), 1);
            assert_eq!(db.segments[&0].name, "CODE");
            assert_eq!(db.segments[&0].ofs, 16);
            assert_eq!(db.scopes.len(), 1);
            assert_eq!(db.scopes[&0].name, "global");
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn load_nested_scope_chain() {
        unsafe {
            let content = "\
scope\tid=0,name=\"A\",size=0
scope\tid=1,name=\"B\",size=0,parent=0
scope\tid=2,name=\"C\",size=0,parent=1
sym\tid=10,name=\"x\",val=0x100,scope=2,type=lab
";
            let path = write_temp_dbg(content);
            let db = load_from_path(path.to_str().unwrap()).unwrap();
            assert_eq!(db.syms.len(), 1);
            let full = db.build_full_scope_name(2);
            assert_eq!(full, "A::B::C::");
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn load_unknown_records_skipped() {
        unsafe {
            let content = "\
mod\tid=0,name=\"foo.o\"
file\tid=0,name=\"foo.s\"
line\tid=0,file=0,line=42
span\tid=0,seg=0,start=0,size=3
sym\tid=99,name=\"only_sym\",val=0xC000,type=lab
";
            let path = write_temp_dbg(content);
            let db = load_from_path(path.to_str().unwrap()).unwrap();
            assert_eq!(db.syms.len(), 1);
            assert_eq!(db.syms[0].name, "only_sym");
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn sym_type_classification() {
        unsafe {
            let content = "\
sym\tid=0,name=\"a\",val=0,type=lab
sym\tid=1,name=\"b\",val=0,type=equ
sym\tid=2,name=\"c\",val=0,type=imp
";
            let path = write_temp_dbg(content);
            let db = load_from_path(path.to_str().unwrap()).unwrap();
            assert_eq!(db.syms[0].sym_type, SymType::Label);
            assert_eq!(db.syms[1].sym_type, SymType::Equ);
            assert_eq!(db.syms[2].sym_type, SymType::Import); // default
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn open_nonexistent_returns_null_and_sets_error() {
        unsafe {
            let bad = CString::new("Z:/this/path/does/not/exist.dbg").unwrap();
            let db = unsafe { fceux11_rust_ld65_open(bad.as_ptr()) };
            assert!(db.is_null());
            let err = fceux11_rust_ld65_last_error();
            assert!(!err.is_null());
        }
    }

    #[test]
    fn ffi_open_iterate_close_roundtrip() {
        unsafe {
            let content = "\
seg\tid=5,name=\"DATA\",ooffs=100
scope\tid=7,name=\"ns\",size=0
sym\tid=42,name=\"label1\",val=0xC0DE,scope=7,seg=5,type=lab
";
            let path = write_temp_dbg(content);
            let cpath = CString::new(path.to_str().unwrap()).unwrap();
            let db = unsafe { fceux11_rust_ld65_open(cpath.as_ptr()) };
            assert!(!db.is_null());
            let count = unsafe { fceux11_rust_ld65_sym_count(db) };
            assert_eq!(count, 1);
            let mut view = FceuLd65Sym::default();
            let ok = unsafe { fceux11_rust_ld65_sym_get(db, 0, &mut view) };
            assert!(ok);
            assert_eq!(view.id, 42);
            assert_eq!(view.value, 0xC0DE);
            assert_eq!(view.sym_type, 1); // LABEL
            assert!(view.has_segment != 0);
            assert_eq!(view.segment_ofs, 100);
            let seg_name = unsafe {
                CStr::from_ptr(view.segment_name_ptr)
                    .to_string_lossy()
                    .into_owned()
            };
            assert_eq!(seg_name, "DATA");
            assert!(view.has_scope != 0);
            let scope_full = unsafe {
                CStr::from_ptr(view.scope_full_name_ptr)
                    .to_string_lossy()
                    .into_owned()
            };
            assert_eq!(scope_full, "ns::");

            unsafe { fceux11_rust_ld65_close(db) };
            let _ = std::fs::remove_file(&path);
        }
    }
}
