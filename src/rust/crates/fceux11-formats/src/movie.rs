//! FM2 movie format parser and serializer.
//!
//! Replaces `LoadFM2`, `FceuMovieData::dump`, `MovieRecord::parse/dump/parseBinary/dumpBinary`
//! from `src/movie.cpp`. C++ retains frame-level state management (`currMovieData`,
//! `FCEUMOV_AddInputState`, etc.); Rust owns the file format logic.
//!
//! # FFI design
//! - **Parsing**: `fceux11_rust_movie_load_fm2` returns an opaque `*mut FceuMovieData`.
//!   C++ extracts fields through getter functions and then frees the handle.
//! - **Serialization**: C++ builds a `FceuMovieDataInput` struct (with slices and
//!   pointers) and calls `fceux11_rust_movie_data_dump`, which writes directly to
//!   an `EmuFileMem` handle.

use std::ffi::{c_char, CStr};
use crate::emufile::EmuFileMem;

// ============================================================
// Internal Rust types
// ============================================================

#[derive(Clone, Default, Debug, PartialEq)]
pub struct MovieRecord {
    pub joysticks: [u8; 4],
    /// (x, y, b, bogo, zaphit) for each zapper port
    pub zappers: [(u8, u8, u8, u8, u64); 2],
    pub commands: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FceuMovieData {
    pub version: i32,
    pub emu_version: i32,
    pub fds: i32,
    pub pal_flag: bool,
    pub ppu_flag: bool,
    pub rom_checksum: [u8; 16],
    pub rom_filename: String,
    pub savestate: Vec<u8>,
    pub saveram: Vec<u8>,
    pub records: Vec<MovieRecord>,
    pub comments: Vec<String>,
    pub subtitles: Vec<String>,
    pub rerecord_count: i32,
    pub guid: String,
    pub binary_flag: bool,
    pub load_frame_count: i32,
    pub ports: [i32; 3],
    pub fourscore: bool,
    pub microphone: bool,
    pub ram_init_option: i32,
    pub ram_init_seed: i32,
}

impl Default for FceuMovieData {
    fn default() -> Self {
        Self {
            version: 0,
            emu_version: 0,
            fds: 0,
            pal_flag: false,
            ppu_flag: false,
            rom_checksum: [0; 16],
            rom_filename: String::new(),
            savestate: Vec::new(),
            saveram: Vec::new(),
            records: Vec::new(),
            comments: Vec::new(),
            subtitles: Vec::new(),
            rerecord_count: 0,
            guid: String::new(),
            binary_flag: false,
            load_frame_count: -1,
            ports: [0; 3],
            fourscore: false,
            microphone: false,
            ram_init_option: 0,
            ram_init_seed: 0,
        }
    }
}

// ============================================================
// Hex / Base64 helpers (minimal, self-contained)
// ============================================================

fn encode_hex(data: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(data.len() * 2);
    for &b in data {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xF) as usize] as char);
    }
    out
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    let bytes: Vec<u8> = s.bytes().filter(|&b| b != b' ' && b != b'\t').collect();
    if bytes.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let hi = hex_digit(chunk[0])?;
        let lo = hex_digit(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode_base64(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= data.len() {
        let b0 = data[i];
        let b1 = data[i + 1];
        let b2 = data[i + 2];
        out.push(BASE64_CHARS[(b0 >> 2) as usize] as char);
        out.push(BASE64_CHARS[(((b0 & 0x3) << 4) | (b1 >> 4)) as usize] as char);
        out.push(BASE64_CHARS[(((b1 & 0xF) << 2) | (b2 >> 6)) as usize] as char);
        out.push(BASE64_CHARS[(b2 & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem > 0 {
        let b0 = data[i];
        let b1 = data.get(i + 1).copied().unwrap_or(0);
        out.push(BASE64_CHARS[(b0 >> 2) as usize] as char);
        out.push(BASE64_CHARS[(((b0 & 0x3) << 4) | (b1 >> 4)) as usize] as char);
        if rem == 2 {
            out.push(BASE64_CHARS[(((b1 & 0xF) << 2)) as usize] as char);
        } else {
            out.push('=');
        }
        out.push('=');
    }
    out
}

fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let bytes: Vec<u8> = s.bytes().filter(|&b| b != b' ' && b != b'\t' && b != b'\n' && b != b'\r').collect();
    if bytes.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut buf = [0u8; 4];
    let mut buf_len = 0;
    for &b in &bytes {
        if b == b'=' {
            break;
        }
        let v = base64_value(b)?;
        buf[buf_len] = v;
        buf_len += 1;
        if buf_len == 4 {
            out.push((buf[0] << 2) | (buf[1] >> 4));
            out.push((buf[1] << 4) | (buf[2] >> 2));
            out.push((buf[2] << 6) | buf[3]);
            buf_len = 0;
        }
    }
    if buf_len >= 2 {
        out.push((buf[0] << 2) | (buf[1] >> 4));
    }
    if buf_len >= 3 {
        out.push((buf[1] << 4) | (buf[2] >> 2));
    }
    Some(out)
}

fn decode_base64_or_hex(s: &str) -> Option<Vec<u8>> {
    decode_base64(s).or_else(|| decode_hex(s))
}

fn base64_value(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

// ============================================================
// Decimal formatting helpers
// ============================================================

fn write_dec_varlen(mut val: u64, out: &mut Vec<u8>) {
    if val == 0 {
        out.push(b'0');
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 0;
    while val > 0 {
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }
    for j in (0..i).rev() {
        out.push(buf[j]);
    }
}

fn write_dec_fixed(mut val: u32, digits: usize, out: &mut Vec<u8>) {
    let mut buf = vec![b'0'; digits];
    for i in 0..digits {
        buf[digits - 1 - i] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    out.extend_from_slice(&buf);
}

const MNEMONICS: &[u8] = b"ABSTUDLR";

fn dump_joy_bits(joy: u8, out: &mut Vec<u8>) {
    for bit in (0..8).rev() {
        let mask = 1 << bit;
        if joy & mask != 0 {
            out.push(MNEMONICS[bit as usize]);
        } else {
            out.push(b'.');
        }
    }
}

// ============================================================
// Text record parsing helpers
// ============================================================

fn parse_joy_bits(data: &[u8], pos: &mut usize) -> u8 {
    let mut joy = 0u8;
    for _ in 0..8 {
        if *pos < data.len() {
            let c = data[*pos];
            joy = (joy << 1) | (if c == b'.' || c == b' ' { 0 } else { 1 });
            *pos += 1;
        } else {
            joy <<= 1;
        }
    }
    joy
}

fn parse_uint_dec(data: &[u8], pos: &mut usize) -> u64 {
    let mut val = 0u64;
    let mut pre = true;
    while *pos < data.len() {
        let c = data[*pos];
        if c >= b'0' && c <= b'9' {
            pre = false;
            val = val * 10 + (c - b'0') as u64;
            *pos += 1;
        } else {
            if !pre {
                break;
            }
            *pos += 1;
        }
    }
    val
}

// ============================================================
// Core FM2 parser
// ============================================================

fn install_value(md: &mut FceuMovieData, key: &str, val: &str) -> Result<(), &'static str> {
    match key {
        "FDS" => md.fds = val.parse().unwrap_or(0),
        "NewPPU" => md.ppu_flag = val.parse::<i32>().unwrap_or(0) != 0,
        "RAMInitOption" => md.ram_init_option = val.parse().unwrap_or(0),
        "RAMInitSeed" => md.ram_init_seed = val.parse().unwrap_or(0),
        "version" => md.version = val.parse().unwrap_or(3),
        "emuVersion" => md.emu_version = val.parse().unwrap_or(0),
        "rerecordCount" => md.rerecord_count = val.parse().unwrap_or(0),
        "palFlag" => md.pal_flag = val.parse::<i32>().unwrap_or(0) != 0,
        "romFilename" => md.rom_filename = val.to_string(),
        "romChecksum" => {
            if let Some(v) = decode_hex(val) {
                let len = v.len().min(16);
                md.rom_checksum[..len].copy_from_slice(&v[..len]);
            }
        }
        "guid" => md.guid = val.to_string(),
        "fourscore" => md.fourscore = val.parse::<i32>().unwrap_or(0) != 0,
        "microphone" => md.microphone = val.parse::<i32>().unwrap_or(0) != 0,
        "port0" => md.ports[0] = val.parse().unwrap_or(0),
        "port1" => md.ports[1] = val.parse().unwrap_or(0),
        "port2" => md.ports[2] = val.parse().unwrap_or(0),
        "binary" => md.binary_flag = val.parse::<i32>().unwrap_or(0) != 0,
        "comment" => md.comments.push(val.to_string()),
        "subtitle" => md.subtitles.push(val.to_string()),
        "savestate" => {
            md.savestate = decode_base64_or_hex(val).unwrap_or_default();
        }
        "saveram" => {
            md.saveram = decode_base64_or_hex(val).unwrap_or_default();
        }
        "length" => md.load_frame_count = val.parse().unwrap_or(-1),
        _ => {}
    }
    Ok(())
}

fn parse_text_record(md: &FceuMovieData, data: &[u8]) -> Result<MovieRecord, &'static str> {
    let mut rec = MovieRecord::default();
    let mut pos = 0usize;

    // commands
    rec.commands = parse_uint_dec(data, &mut pos) as u8;

    // Expect '|'
    if pos >= data.len() || data[pos] != b'|' {
        return Err("invalid movie record: missing pipe after commands");
    }
    pos += 1;

    if md.fourscore {
        for i in 0..4 {
            rec.joysticks[i] = parse_joy_bits(data, &mut pos);
            if pos >= data.len() || data[pos] != b'|' {
                return Err("invalid fourscore record");
            }
            pos += 1;
        }
    } else {
        for port in 0..2 {
            match md.ports[port] {
                0 => {
                    // SI_GAMEPAD
                    rec.joysticks[port] = parse_joy_bits(data, &mut pos);
                }
                1 => {
                    // SI_ZAPPER
                    rec.zappers[port].0 = parse_uint_dec(data, &mut pos) as u8;
                    rec.zappers[port].1 = parse_uint_dec(data, &mut pos) as u8;
                    rec.zappers[port].2 = parse_uint_dec(data, &mut pos) as u8;
                    rec.zappers[port].3 = parse_uint_dec(data, &mut pos) as u8;
                    rec.zappers[port].4 = parse_uint_dec(data, &mut pos);
                }
                _ => {}
            }
            if pos >= data.len() || data[pos] != b'|' {
                return Err("invalid record: missing pipe after port");
            }
            pos += 1;
        }
    }

    // fcexp pipe (no data logged) — pos is not consumed past this function.
    Ok(rec)
}

fn parse_binary_chunk(md: &mut FceuMovieData, data: &[u8]) -> Result<(), &'static str> {
    let mut record_size = 1usize; // 1 for commands
    if md.fourscore {
        record_size += 4;
    } else {
        for i in 0..2 {
            match md.ports[i] {
                0 => record_size += 1,  // SI_GAMEPAD
                1 => record_size += 12, // SI_ZAPPER
                _ => {}
            }
        }
    }

    if record_size == 0 {
        return Ok(());
    }

    let mut num_records = data.len() / record_size;
    if md.load_frame_count >= 0 && (md.load_frame_count as usize) < num_records {
        num_records = md.load_frame_count as usize;
    }

    let mut pos = 0usize;
    for _ in 0..num_records {
        let mut rec = MovieRecord::default();
        if pos >= data.len() {
            break;
        }
        rec.commands = data[pos];
        pos += 1;

        if md.fourscore {
            for i in 0..4 {
                if pos < data.len() {
                    rec.joysticks[i] = data[pos];
                    pos += 1;
                }
            }
        } else {
            for port in 0..2 {
                match md.ports[port] {
                    0 => {
                        if pos < data.len() {
                            rec.joysticks[port] = data[pos];
                            pos += 1;
                        }
                    }
                    1 => {
                        if pos + 12 <= data.len() {
                            rec.zappers[port].0 = data[pos];
                            rec.zappers[port].1 = data[pos + 1];
                            rec.zappers[port].2 = data[pos + 2];
                            rec.zappers[port].3 = data[pos + 3];
                            rec.zappers[port].4 = u64::from_le_bytes([
                                data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7],
                                data[pos + 8], data[pos + 9], data[pos + 10], data[pos + 11],
                            ]);
                            pos += 12;
                        }
                    }
                    _ => {}
                }
            }
        }
        md.records.push(rec);
    }

    Ok(())
}

fn load_fm2(data: &[u8], stop_after_header: bool) -> Result<FceuMovieData, &'static str> {
    let mut md = FceuMovieData::default();
    md.version = 3;
    md.load_frame_count = -1;

    // Check FCM signature
    if data.len() >= 3 && &data[..3] == b"FCM" {
        return Err("FCM format no longer supported");
    }

    // Must start with "version 3"
    if data.len() < 9 || &data[..9] != b"version 3" {
        return Err("not a valid FM2 file");
    }

    let len = data.len();
    let mut pos = 0usize;

    while pos < len {
        // Skip empty lines
        while pos < len && (data[pos] == b'\n' || data[pos] == b'\r') {
            pos += 1;
        }
        if pos >= len {
            break;
        }

        // Record line?
        if data[pos] == b'|' {
            if stop_after_header {
                return Ok(md);
            }
            if md.binary_flag && !stop_after_header {
                pos += 1; // skip '|'
                parse_binary_chunk(&mut md, &data[pos..])?;
                return Ok(md);
            }
            let line_start = pos;
            while pos < len && data[pos] != b'\n' && data[pos] != b'\r' {
                pos += 1;
            }
            let line = &data[line_start..pos];
            let rec = parse_text_record(&md, &line[1..])?; // skip leading '|'
            md.records.push(rec);

            if md.load_frame_count >= 0 && md.records.len() as i32 == md.load_frame_count {
                return Ok(md);
            }
            continue;
        }

        // Key-value line
        let line_start = pos;
        while pos < len && data[pos] != b'\n' && data[pos] != b'\r' {
            pos += 1;
        }
        let line = &data[line_start..pos];

        // Split at first whitespace (space or tab)
        let mut split = line.splitn(2, |&b| b == b' ' || b == b'\t');
        let key_bytes = split.next().unwrap_or(&[]);
        let value_bytes = split.next().unwrap_or(&[]);
        let key = String::from_utf8_lossy(key_bytes);
        let value = String::from_utf8_lossy(value_bytes);
        install_value(&mut md, &key, &value)?;

        if md.load_frame_count >= 0 && md.records.len() as i32 == md.load_frame_count {
            return Ok(md);
        }
    }

    Ok(md)
}

// ============================================================
// Core FM2 serializer
// ============================================================

#[allow(dead_code)]
fn dump_text_record_to_vec(md: &FceuMovieData, rec: &MovieRecord, _index: usize, out: &mut Vec<u8>) {
    out.push(b'|');
    write_dec_varlen(rec.commands as u64, out);
    out.push(b'|');

    if md.fourscore {
        for i in 0..4 {
            dump_joy_bits(rec.joysticks[i], out);
            out.push(b'|');
        }
    } else {
        for port in 0..2 {
            match md.ports[port] {
                0 => {
                    dump_joy_bits(rec.joysticks[port], out);
                }
                1 => {
                    write_dec_fixed(rec.zappers[port].0 as u32, 3, out);
                    out.push(b' ');
                    write_dec_fixed(rec.zappers[port].1 as u32, 3, out);
                    out.push(b' ');
                    write_dec_fixed(rec.zappers[port].2 as u32, 1, out);
                    out.push(b' ');
                    write_dec_fixed(rec.zappers[port].3 as u32, 1, out);
                    out.push(b' ');
                    write_dec_varlen(rec.zappers[port].4, out);
                }
                _ => {}
            }
            out.push(b'|');
        }
    }

    out.push(b'|'); // fcexp
    out.push(b'\n');
}

#[allow(dead_code)]
fn dump_binary_record_to_vec(md: &FceuMovieData, rec: &MovieRecord, out: &mut Vec<u8>) {
    out.push(rec.commands);

    if md.fourscore {
        for i in 0..4 {
            out.push(rec.joysticks[i]);
        }
    } else {
        for port in 0..2 {
            match md.ports[port] {
                0 => {
                    out.push(rec.joysticks[port]);
                }
                1 => {
                    out.push(rec.zappers[port].0);
                    out.push(rec.zappers[port].1);
                    out.push(rec.zappers[port].2);
                    out.push(rec.zappers[port].3);
                    out.extend_from_slice(&rec.zappers[port].4.to_le_bytes());
                }
                _ => {}
            }
        }
    }
}

#[allow(dead_code)]
fn dump_fm2(md: &FceuMovieData, binary: bool) -> Vec<u8> {
    let mut out = Vec::new();

    // Header
    out.extend_from_slice(format!("version {}\n", md.version).as_bytes());
    out.extend_from_slice(format!("emuVersion {}\n", md.emu_version).as_bytes());
    out.extend_from_slice(format!("rerecordCount {}\n", md.rerecord_count).as_bytes());
    out.extend_from_slice(format!("palFlag {}\n", if md.pal_flag { 1 } else { 0 }).as_bytes());
    out.extend_from_slice(format!("romFilename {}\n", md.rom_filename).as_bytes());
    out.extend_from_slice(format!("romChecksum {}\n", encode_hex(&md.rom_checksum)).as_bytes());
    out.extend_from_slice(format!("guid {}\n", md.guid).as_bytes());
    out.extend_from_slice(format!("fourscore {}\n", if md.fourscore { 1 } else { 0 }).as_bytes());
    out.extend_from_slice(format!("microphone {}\n", if md.microphone { 1 } else { 0 }).as_bytes());
    out.extend_from_slice(format!("port0 {}\n", md.ports[0]).as_bytes());
    out.extend_from_slice(format!("port1 {}\n", md.ports[1]).as_bytes());
    out.extend_from_slice(format!("port2 {}\n", md.ports[2]).as_bytes());
    out.extend_from_slice(format!("FDS {}\n", if md.fds != 0 { 1 } else { 0 }).as_bytes());
    out.extend_from_slice(format!("NewPPU {}\n", if md.ppu_flag { 1 } else { 0 }).as_bytes());
    out.extend_from_slice(format!("RAMInitOption {}\n", md.ram_init_option).as_bytes());
    out.extend_from_slice(format!("RAMInitSeed {}\n", md.ram_init_seed).as_bytes());

    for comment in &md.comments {
        out.extend_from_slice(format!("comment {}\n", comment).as_bytes());
    }
    for subtitle in &md.subtitles {
        out.extend_from_slice(format!("subtitle {}\n", subtitle).as_bytes());
    }

    if binary {
        out.extend_from_slice(b"binary 1\n");
    }

    if !md.savestate.is_empty() {
        out.extend_from_slice(format!("savestate {}\n", encode_base64(&md.savestate)).as_bytes());
    }
    if !md.saveram.is_empty() {
        out.extend_from_slice(format!("saveram {}\n", encode_base64(&md.saveram)).as_bytes());
    }

    if md.load_frame_count >= 0 {
        out.extend_from_slice(format!("length {}\n", md.load_frame_count).as_bytes());
    }

    // Records
    if binary {
        out.push(b'|');
        for rec in &md.records {
            dump_binary_record_to_vec(md, rec, &mut out);
        }
    } else {
        for (i, rec) in md.records.iter().enumerate() {
            dump_text_record_to_vec(md, rec, i, &mut out);
        }
    }

    out
}


// ============================================================
// FFI: C-compatible types
// ============================================================

/// C-compatible movie record layout.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FceuMovieRecord {
    pub joysticks: [u8; 4],
    pub zapper_x: [u8; 2],
    pub zapper_y: [u8; 2],
    pub zapper_b: [u8; 2],
    pub zapper_bogo: [u8; 2],
    pub zapper_zaphit: [u64; 2],
    pub commands: u8,
}

impl Default for FceuMovieRecord {
    fn default() -> Self {
        Self {
            joysticks: [0; 4],
            zapper_x: [0; 2],
            zapper_y: [0; 2],
            zapper_b: [0; 2],
            zapper_bogo: [0; 2],
            zapper_zaphit: [0; 2],
            commands: 0,
        }
    }
}

impl From<&MovieRecord> for FceuMovieRecord {
    fn from(r: &MovieRecord) -> Self {
        Self {
            joysticks: r.joysticks,
            zapper_x: [r.zappers[0].0, r.zappers[1].0],
            zapper_y: [r.zappers[0].1, r.zappers[1].1],
            zapper_b: [r.zappers[0].2, r.zappers[1].2],
            zapper_bogo: [r.zappers[0].3, r.zappers[1].3],
            zapper_zaphit: [r.zappers[0].4, r.zappers[1].4],
            commands: r.commands,
        }
    }
}

impl From<&FceuMovieRecord> for MovieRecord {
    fn from(r: &FceuMovieRecord) -> Self {
        Self {
            joysticks: r.joysticks,
            zappers: [
                (r.zapper_x[0], r.zapper_y[0], r.zapper_b[0], r.zapper_bogo[0], r.zapper_zaphit[0]),
                (r.zapper_x[1], r.zapper_y[1], r.zapper_b[1], r.zapper_bogo[1], r.zapper_zaphit[1]),
            ],
            commands: r.commands,
        }
    }
}

/// C-compatible input descriptor for dumping a movie.
/// All pointer fields may be null / zero when unused.
#[repr(C)]
pub struct FceuMovieDataInput {
    pub version: i32,
    pub emu_version: i32,
    pub fds: i32,
    pub pal_flag: bool,
    pub ppu_flag: bool,
    pub rom_checksum: [u8; 16],
    pub rom_filename: *const c_char,
    pub savestate: *const u8,
    pub savestate_len: usize,
    pub saveram: *const u8,
    pub saveram_len: usize,
    pub records: *const FceuMovieRecord,
    pub records_count: usize,
    pub comments: *const *const c_char,
    pub comments_count: usize,
    pub subtitles: *const *const c_char,
    pub subtitles_count: usize,
    pub rerecord_count: i32,
    pub guid: *const c_char,
    pub binary_flag: bool,
    pub load_frame_count: i32,
    pub ports: [i32; 3],
    pub fourscore: bool,
    pub microphone: bool,
    pub ram_init_option: i32,
    pub ram_init_seed: i32,
}

fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

fn movie_data_from_input(input: &FceuMovieDataInput) -> FceuMovieData {
    let mut md = FceuMovieData::default();
    md.version = input.version;
    md.emu_version = input.emu_version;
    md.fds = input.fds;
    md.pal_flag = input.pal_flag;
    md.ppu_flag = input.ppu_flag;
    md.rom_checksum = input.rom_checksum;
    md.rom_filename = cstr_to_string(input.rom_filename);
    md.rerecord_count = input.rerecord_count;
    md.guid = cstr_to_string(input.guid);
    md.binary_flag = input.binary_flag;
    md.load_frame_count = input.load_frame_count;
    md.ports = input.ports;
    md.fourscore = input.fourscore;
    md.microphone = input.microphone;
    md.ram_init_option = input.ram_init_option;
    md.ram_init_seed = input.ram_init_seed;

    if !input.savestate.is_null() && input.savestate_len > 0 {
        md.savestate = unsafe {
            std::slice::from_raw_parts(input.savestate, input.savestate_len).to_vec()
        };
    }
    if !input.saveram.is_null() && input.saveram_len > 0 {
        md.saveram = unsafe {
            std::slice::from_raw_parts(input.saveram, input.saveram_len).to_vec()
        };
    }

    if !input.records.is_null() && input.records_count > 0 {
        let slice = unsafe { std::slice::from_raw_parts(input.records, input.records_count) };
        md.records = slice.iter().map(|r| MovieRecord::from(r)).collect();
    }

    if !input.comments.is_null() && input.comments_count > 0 {
        let slice = unsafe { std::slice::from_raw_parts(input.comments, input.comments_count) };
        md.comments = slice.iter().map(|&p| cstr_to_string(p)).collect();
    }

    if !input.subtitles.is_null() && input.subtitles_count > 0 {
        let slice = unsafe { std::slice::from_raw_parts(input.subtitles, input.subtitles_count) };
        md.subtitles = slice.iter().map(|&p| cstr_to_string(p)).collect();
    }

    md
}

// ============================================================
// FFI: Opaque handle functions
// ============================================================

/// Parse an FM2 file from raw bytes.
/// Returns an opaque handle to `FceuMovieData`, or null on error.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_movie_load_fm2(
    data: *const u8,
    len: usize,
    stop_after_header: bool,
) -> *mut FceuMovieData {
    if data.is_null() || len == 0 {
        return std::ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    match load_fm2(slice, stop_after_header) {
        Ok(md) => Box::into_raw(Box::new(md)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a `FceuMovieData` handle obtained from `fceux11_rust_movie_load_fm2`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_free(md: *mut FceuMovieData) {
    if !md.is_null() {
        unsafe { drop(Box::from_raw(md)) };
    }
}

// --- Scalar getters ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_version(md: *const FceuMovieData) -> i32 {
    unsafe { md.as_ref() }.map(|m| m.version).unwrap_or(3)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_emu_version(md: *const FceuMovieData) -> i32 {
    unsafe { md.as_ref() }.map(|m| m.emu_version).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_fds(md: *const FceuMovieData) -> i32 {
    unsafe { md.as_ref() }.map(|m| m.fds).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_pal_flag(md: *const FceuMovieData) -> bool {
    unsafe { md.as_ref() }.map(|m| m.pal_flag).unwrap_or(false)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_ppu_flag(md: *const FceuMovieData) -> bool {
    unsafe { md.as_ref() }.map(|m| m.ppu_flag).unwrap_or(false)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_rerecord_count(md: *const FceuMovieData) -> i32 {
    unsafe { md.as_ref() }.map(|m| m.rerecord_count).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_binary_flag(md: *const FceuMovieData) -> bool {
    unsafe { md.as_ref() }.map(|m| m.binary_flag).unwrap_or(false)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_load_frame_count(md: *const FceuMovieData) -> i32 {
    unsafe { md.as_ref() }.map(|m| m.load_frame_count).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_fourscore(md: *const FceuMovieData) -> bool {
    unsafe { md.as_ref() }.map(|m| m.fourscore).unwrap_or(false)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_microphone(md: *const FceuMovieData) -> bool {
    unsafe { md.as_ref() }.map(|m| m.microphone).unwrap_or(false)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_ram_init_option(md: *const FceuMovieData) -> i32 {
    unsafe { md.as_ref() }.map(|m| m.ram_init_option).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_ram_init_seed(md: *const FceuMovieData) -> i32 {
    unsafe { md.as_ref() }.map(|m| m.ram_init_seed).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_ports(md: *const FceuMovieData, out: *mut i32) {
    if let (Some(m), Some(o)) = (unsafe { md.as_ref() }, unsafe { out.as_mut() }) {
        let dst = unsafe { std::slice::from_raw_parts_mut(o, 3) };
        dst.copy_from_slice(&m.ports);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_rom_checksum(
    md: *const FceuMovieData,
    out: *mut u8,
) {
    if let (Some(m), Some(o)) = (unsafe { md.as_ref() }, unsafe { out.as_mut() }) {
        let dst = unsafe { std::slice::from_raw_parts_mut(o, 16) };
        dst.copy_from_slice(&m.rom_checksum);
    }
}

// --- String getters (thread_local buffer) ---

fn with_thread_local_buf<F: FnOnce(&mut [u8; 512]) -> *const c_char>(s: &str, f: F) -> *const c_char {
    thread_local! {
        static BUF: std::cell::RefCell<[u8; 512]> = std::cell::RefCell::new([0u8; 512]);
    }
    BUF.with(|buf| {
        let mut b = buf.borrow_mut();
        let bytes = s.as_bytes();
        let len = bytes.len().min(511);
        b[..len].copy_from_slice(&bytes[..len]);
        b[len] = 0;
        f(&mut *b)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_rom_filename(md: *const FceuMovieData) -> *const c_char {
    let s = unsafe { md.as_ref() }.map(|m| m.rom_filename.as_str()).unwrap_or("");
    with_thread_local_buf(s, |b| b.as_ptr() as *const c_char)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_guid(md: *const FceuMovieData) -> *const c_char {
    let s = unsafe { md.as_ref() }.map(|m| m.guid.as_str()).unwrap_or("");
    with_thread_local_buf(s, |b| b.as_ptr() as *const c_char)
}

// --- Vector getters ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_records_count(md: *const FceuMovieData) -> usize {
    unsafe { md.as_ref() }.map(|m| m.records.len()).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_record_get(
    md: *const FceuMovieData,
    index: usize,
    out: *mut FceuMovieRecord,
) -> bool {
    if let (Some(m), Some(o)) = (unsafe { md.as_ref() }, unsafe { out.as_mut() }) {
        if let Some(rec) = m.records.get(index) {
            *o = FceuMovieRecord::from(rec);
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_savestate_len(md: *const FceuMovieData) -> usize {
    unsafe { md.as_ref() }.map(|m| m.savestate.len()).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_savestate_copy(
    md: *const FceuMovieData,
    out: *mut u8,
    out_len: usize,
) -> usize {
    if let (Some(m), Some(o)) = (unsafe { md.as_ref() }, unsafe { out.as_mut() }) {
        let len = m.savestate.len().min(out_len);
        let dst = unsafe { std::slice::from_raw_parts_mut(o, len) };
        dst.copy_from_slice(&m.savestate[..len]);
        return len;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_saveram_len(md: *const FceuMovieData) -> usize {
    unsafe { md.as_ref() }.map(|m| m.saveram.len()).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_saveram_copy(
    md: *const FceuMovieData,
    out: *mut u8,
    out_len: usize,
) -> usize {
    if let (Some(m), Some(o)) = (unsafe { md.as_ref() }, unsafe { out.as_mut() }) {
        let len = m.saveram.len().min(out_len);
        let dst = unsafe { std::slice::from_raw_parts_mut(o, len) };
        dst.copy_from_slice(&m.saveram[..len]);
        return len;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_comments_count(md: *const FceuMovieData) -> usize {
    unsafe { md.as_ref() }.map(|m| m.comments.len()).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_comment_get(
    md: *const FceuMovieData,
    index: usize,
) -> *const c_char {
    let s = unsafe { md.as_ref() }
        .and_then(|m| m.comments.get(index))
        .map(|s| s.as_str())
        .unwrap_or("");
    with_thread_local_buf(s, |b| b.as_ptr() as *const c_char)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_subtitles_count(md: *const FceuMovieData) -> usize {
    unsafe { md.as_ref() }.map(|m| m.subtitles.len()).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_subtitle_get(
    md: *const FceuMovieData,
    index: usize,
) -> *const c_char {
    let s = unsafe { md.as_ref() }
        .and_then(|m| m.subtitles.get(index))
        .map(|s| s.as_str())
        .unwrap_or("");
    with_thread_local_buf(s, |b| b.as_ptr() as *const c_char)
}

// ============================================================
// FFI: Dump
// ============================================================

/// Dump movie data to an `EmuFileMem` handle.
/// Returns the number of bytes written, or -1 on error.
/// If `seek_to_curr_frame_pos` is true and a frame position is recorded,
/// `out_curr_frame_pos` is set to that position; otherwise it is set to -1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_movie_data_dump(
    input: *const FceuMovieDataInput,
    out_handle: *mut EmuFileMem,
    binary: bool,
    seek_to_curr_frame_pos: bool,
    curr_frame_counter: i32,
    out_curr_frame_pos: *mut i32,
) -> i32 {
    if input.is_null() || out_handle.is_null() {
        return -1;
    }
    let input = unsafe { &*input };
    let out = unsafe { &mut *out_handle };
    let md = movie_data_from_input(input);

    let start = out.tell();
    let mut curr_frame_pos: Option<usize> = None;

    // Header
    let header = format!("version {}\n", md.version);
    out.write_from(header.as_ptr(), header.len());
    let header = format!("emuVersion {}\n", md.emu_version);
    out.write_from(header.as_ptr(), header.len());
    let header = format!("rerecordCount {}\n", md.rerecord_count);
    out.write_from(header.as_ptr(), header.len());
    let header = format!("palFlag {}\n", if md.pal_flag { 1 } else { 0 });
    out.write_from(header.as_ptr(), header.len());
    let header = format!("romFilename {}\n", md.rom_filename);
    out.write_from(header.as_ptr(), header.len());
    let header = format!("romChecksum {}\n", encode_hex(&md.rom_checksum));
    out.write_from(header.as_ptr(), header.len());
    let header = format!("guid {}\n", md.guid);
    out.write_from(header.as_ptr(), header.len());
    let header = format!("fourscore {}\n", if md.fourscore { 1 } else { 0 });
    out.write_from(header.as_ptr(), header.len());
    let header = format!("microphone {}\n", if md.microphone { 1 } else { 0 });
    out.write_from(header.as_ptr(), header.len());
    let header = format!("port0 {}\n", md.ports[0]);
    out.write_from(header.as_ptr(), header.len());
    let header = format!("port1 {}\n", md.ports[1]);
    out.write_from(header.as_ptr(), header.len());
    let header = format!("port2 {}\n", md.ports[2]);
    out.write_from(header.as_ptr(), header.len());
    let header = format!("FDS {}\n", if md.fds != 0 { 1 } else { 0 });
    out.write_from(header.as_ptr(), header.len());
    let header = format!("NewPPU {}\n", if md.ppu_flag { 1 } else { 0 });
    out.write_from(header.as_ptr(), header.len());
    let header = format!("RAMInitOption {}\n", md.ram_init_option);
    out.write_from(header.as_ptr(), header.len());
    let header = format!("RAMInitSeed {}\n", md.ram_init_seed);
    out.write_from(header.as_ptr(), header.len());

    for comment in &md.comments {
        let line = format!("comment {}\n", comment);
        out.write_from(line.as_ptr(), line.len());
    }
    for subtitle in &md.subtitles {
        let line = format!("subtitle {}\n", subtitle);
        out.write_from(line.as_ptr(), line.len());
    }

    if binary {
        out.write_from(b"binary 1\n".as_ptr(), 9);
    }

    if !md.savestate.is_empty() {
        let line = format!("savestate {}\n", encode_base64(&md.savestate));
        out.write_from(line.as_ptr(), line.len());
    }
    if !md.saveram.is_empty() {
        let line = format!("saveram {}\n", encode_base64(&md.saveram));
        out.write_from(line.as_ptr(), line.len());
    }

    if md.load_frame_count >= 0 {
        let line = format!("length {}\n", md.load_frame_count);
        out.write_from(line.as_ptr(), line.len());
    }

    // Records
    if binary {
        out.write_from(b"|".as_ptr(), 1);
        for (i, rec) in md.records.iter().enumerate() {
            if seek_to_curr_frame_pos && curr_frame_counter == i as i32 {
                curr_frame_pos = Some(out.tell());
            }
            dump_binary_record_to_emufile(&md, rec, out);
        }
    } else {
        for (i, rec) in md.records.iter().enumerate() {
            if seek_to_curr_frame_pos && curr_frame_counter == i as i32 {
                curr_frame_pos = Some(out.tell());
            }
            dump_text_record_to_emufile(&md, rec, i, out);
        }
    }

    let end = out.tell();
    if !out_curr_frame_pos.is_null() {
        unsafe {
            *out_curr_frame_pos = curr_frame_pos.map(|p| p as i32).unwrap_or(-1);
        }
    }
    if let Some(pos) = curr_frame_pos {
        out.seek_set(pos as isize);
    }

    (end - start) as i32
}

fn dump_text_record_to_emufile(md: &FceuMovieData, rec: &MovieRecord, _index: usize, out: &mut EmuFileMem) {
    out.write_from(b"|".as_ptr(), 1);
    let mut buf = Vec::new();
    write_dec_varlen(rec.commands as u64, &mut buf);
    out.write_from(buf.as_ptr(), buf.len());
    out.write_from(b"|".as_ptr(), 1);

    if md.fourscore {
        for i in 0..4 {
            buf.clear();
            dump_joy_bits(rec.joysticks[i], &mut buf);
            out.write_from(buf.as_ptr(), buf.len());
            out.write_from(b"|".as_ptr(), 1);
        }
    } else {
        for port in 0..2 {
            match md.ports[port] {
                0 => {
                    buf.clear();
                    dump_joy_bits(rec.joysticks[port], &mut buf);
                    out.write_from(buf.as_ptr(), buf.len());
                }
                1 => {
                    buf.clear();
                    write_dec_fixed(rec.zappers[port].0 as u32, 3, &mut buf);
                    out.write_from(buf.as_ptr(), buf.len());
                    out.write_from(b" ".as_ptr(), 1);
                    buf.clear();
                    write_dec_fixed(rec.zappers[port].1 as u32, 3, &mut buf);
                    out.write_from(buf.as_ptr(), buf.len());
                    out.write_from(b" ".as_ptr(), 1);
                    buf.clear();
                    write_dec_fixed(rec.zappers[port].2 as u32, 1, &mut buf);
                    out.write_from(buf.as_ptr(), buf.len());
                    out.write_from(b" ".as_ptr(), 1);
                    buf.clear();
                    write_dec_fixed(rec.zappers[port].3 as u32, 1, &mut buf);
                    out.write_from(buf.as_ptr(), buf.len());
                    out.write_from(b" ".as_ptr(), 1);
                    buf.clear();
                    write_dec_varlen(rec.zappers[port].4, &mut buf);
                    out.write_from(buf.as_ptr(), buf.len());
                }
                _ => {}
            }
            out.write_from(b"|".as_ptr(), 1);
        }
    }

    out.write_from(b"|".as_ptr(), 1); // fcexp
    out.write_from(b"\n".as_ptr(), 1);
}

fn dump_binary_record_to_emufile(md: &FceuMovieData, rec: &MovieRecord, out: &mut EmuFileMem) {
    out.write_from(&rec.commands, 1);

    if md.fourscore {
        for i in 0..4 {
            out.write_from(&rec.joysticks[i], 1);
        }
    } else {
        for port in 0..2 {
            match md.ports[port] {
                0 => {
                    out.write_from(&rec.joysticks[port], 1);
                }
                1 => {
                    out.write_from(&rec.zappers[port].0, 1);
                    out.write_from(&rec.zappers[port].1, 1);
                    out.write_from(&rec.zappers[port].2, 1);
                    out.write_from(&rec.zappers[port].3, 1);
                    let bytes = rec.zappers[port].4.to_le_bytes();
                    out.write_from(bytes.as_ptr(), 8);
                }
                _ => {}
            }
        }
    }
}


// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_movie() -> FceuMovieData {
        let mut md = FceuMovieData::default();
        md.version = 3;
        md.emu_version = 9813;
        md.rerecord_count = 42;
        md.pal_flag = false;
        md.ppu_flag = false;
        md.rom_filename = "test.nes".to_string();
        md.rom_checksum = [0xAB; 16];
        md.guid = "12345678-1234-1234-1234-123456789ABC".to_string();
        md.fourscore = false;
        md.microphone = false;
        md.ports = [0, 1, 0]; // port0=gamepad, port1=zapper
        md.fds = 0;
        md.ram_init_option = 0;
        md.ram_init_seed = 0;

        let mut rec1 = MovieRecord::default();
        rec1.joysticks[0] = 0b10010001; // A + Start + Right
        rec1.commands = 0;
        md.records.push(rec1);

        let mut rec2 = MovieRecord::default();
        rec2.joysticks[0] = 0b01001010; // B + Select + Left + Up
        rec2.commands = 0;
        md.records.push(rec2);

        md
    }

    #[test]
    fn test_hex_roundtrip() {
        let data = [0xABu8, 0xCD, 0xEF, 0x01];
        let s = encode_hex(&data);
        assert_eq!(s, "ABCDEF01");
        let decoded = decode_hex(&s).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = b"Hello World! This is a test.";
        let s = encode_base64(data);
        let decoded = decode_base64(&s).unwrap();
        assert_eq!(decoded, data.as_slice());
    }

    #[test]
    fn test_base64_empty() {
        let s = encode_base64(b"");
        assert_eq!(s, "");
        assert_eq!(decode_base64(&s).unwrap(), b"".as_slice());
    }

    #[test]
    fn test_parse_joy_bits() {
        // C++ mnemonics[8] = {'A','B','S','T','U','D','L','R'}
        // dumpJoy outputs bit7..bit0, so position0='R'(bit7), pos1='L'(bit6), etc.
        let data = b"R.D..S.A";
        let mut pos = 0;
        let joy = parse_joy_bits(data, &mut pos);
        // R=1(bit7), .=0(bit6), D=1(bit5), .=0(bit4), .=0(bit3), S=1(bit2), .=0(bit1), A=1(bit0)
        // binary: 1 0 1 0 0 1 0 1 = 0xA5
        assert_eq!(joy, 0xA5);
        assert_eq!(pos, 8);
    }

    #[test]
    fn test_parse_uint_dec() {
        let data = b"123|";
        let mut pos = 0;
        assert_eq!(parse_uint_dec(data, &mut pos), 123);
        assert_eq!(pos, 3);
    }

    #[test]
    fn test_dump_joy_bits() {
        let mut out = Vec::new();
        dump_joy_bits(0xA5, &mut out);
        // 0xA5 = 0b10100101 -> R.D..S.A
        assert_eq!(&out, b"R.D..S.A");
    }

    #[test]
    fn test_text_record_roundtrip() {
        let mut md = FceuMovieData::default();
        md.fourscore = false;
        md.ports = [0, 0, 0];

        let mut rec = MovieRecord::default();
        rec.joysticks[0] = 0b10010001; // bit7=1(R), bit4=1(U), bit0=1(A)
        rec.joysticks[1] = 0b01001010; // bit6=1(L), bit3=1(T), bit1=1(B)
        rec.commands = 5;

        let mut buf = Vec::new();
        dump_text_record_to_vec(&md, &rec, 0, &mut buf);
        let line = String::from_utf8(buf).unwrap();
        // bit7..bit0 -> R..U...A and .L..T.B.
        assert_eq!(line, "|5|R..U...A|.L..T.B.||\n");

        // Parse it back
        let parsed = parse_text_record(&md, b"5|R..U...A|.L..T.B.|").unwrap();
        assert_eq!(parsed.commands, 5);
        assert_eq!(parsed.joysticks[0], 0b10010001);
        assert_eq!(parsed.joysticks[1], 0b01001010);
    }

    #[test]
    fn test_fm2_dump_and_load_text() {
        let md = make_test_movie();
        let dumped = dump_fm2(&md, false);
        let dumped_str = String::from_utf8(dumped.clone()).unwrap();
        assert!(dumped_str.contains("version 3\n"));
        assert!(dumped_str.contains("romFilename test.nes\n"));
        // port0=gamepad, port1=zapper -> first record joy0=R..U...A, zapper=000 000 0 0 0
        assert!(dumped_str.contains("|0|R..U...A|000 000 0 0 0|"));

        let loaded = load_fm2(&dumped, false).unwrap();
        assert_eq!(loaded.version, md.version);
        assert_eq!(loaded.emu_version, md.emu_version);
        assert_eq!(loaded.rerecord_count, md.rerecord_count);
        assert_eq!(loaded.rom_filename, md.rom_filename);
        assert_eq!(loaded.rom_checksum, md.rom_checksum);
        assert_eq!(loaded.guid, md.guid);
        assert_eq!(loaded.records.len(), md.records.len());
        assert_eq!(loaded.records[0].joysticks[0], md.records[0].joysticks[0]);
    }

    #[test]
    fn test_fm2_binary_roundtrip() {
        let mut md = make_test_movie();
        md.binary_flag = true;
        md.fourscore = false;
        md.ports = [0, 0, 0];

        // Add a third record with zapper data (ports=[0,0,0] so both gamepads)
        let mut rec3 = MovieRecord::default();
        rec3.joysticks[0] = 0x55;
        rec3.joysticks[1] = 0xAA;
        md.records.push(rec3);

        let dumped = dump_fm2(&md, true);
        let loaded = load_fm2(&dumped, false).unwrap();
        assert_eq!(loaded.records.len(), md.records.len());
        assert_eq!(loaded.binary_flag, true);
        assert_eq!(loaded.records[2].joysticks[0], 0x55);
        assert_eq!(loaded.records[2].joysticks[1], 0xAA);
    }

    #[test]
    fn test_fm2_stop_after_header() {
        let md = make_test_movie();
        let dumped = dump_fm2(&md, false);
        let loaded = load_fm2(&dumped, true).unwrap();
        assert_eq!(loaded.records.len(), 0);
        assert_eq!(loaded.rom_filename, "test.nes");
    }

    #[test]
    fn test_fm2_load_frame_count() {
        let mut md = make_test_movie();
        md.load_frame_count = 1;
        let dumped = dump_fm2(&md, false);
        let loaded = load_fm2(&dumped, false).unwrap();
        assert_eq!(loaded.records.len(), 1);
    }

    #[test]
    fn test_fm2_comments_and_subtitles() {
        let mut md = make_test_movie();
        md.comments.push("author test".to_string());
        md.subtitles.push("60 Hello".to_string());
        let dumped = dump_fm2(&md, false);
        let loaded = load_fm2(&dumped, false).unwrap();
        assert_eq!(loaded.comments.len(), 1);
        assert_eq!(loaded.comments[0], "author test");
        assert_eq!(loaded.subtitles.len(), 1);
        assert_eq!(loaded.subtitles[0], "60 Hello");
    }

    #[test]
    fn test_fm2_savestate_roundtrip() {
        let mut md = make_test_movie();
        md.savestate = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let dumped = dump_fm2(&md, false);
        let loaded = load_fm2(&dumped, false).unwrap();
        assert_eq!(loaded.savestate, md.savestate);
    }

    #[test]
    fn test_invalid_fm2_fcm() {
        let data = b"FCM\x00version 3\n";
        assert!(load_fm2(data, false).is_err());
    }

    #[test]
    fn test_invalid_fm2_no_version() {
        let data = b"hello world\n";
        assert!(load_fm2(data, false).is_err());
    }

    #[test]
    fn test_fourscore_record() {
        let mut md = FceuMovieData::default();
        md.fourscore = true;
        md.ports = [0, 0, 0];

        let mut rec = MovieRecord::default();
        // 0x81=R......A, 0x42=.L....B., 0x24=..D..S.., 0x18=...UT...
        rec.joysticks = [0x81, 0x42, 0x24, 0x18];
        rec.commands = 0;

        let mut buf = Vec::new();
        dump_text_record_to_vec(&md, &rec, 0, &mut buf);
        let line = String::from_utf8(buf).unwrap();
        assert_eq!(line, "|0|R......A|.L....B.|..D..S..|...UT...||\n");

        let parsed = parse_text_record(&md, b"0|R......A|.L....B.|..D..S..|...UT...|").unwrap();
        assert_eq!(parsed.joysticks, [0x81, 0x42, 0x24, 0x18]);
    }

    #[test]
    fn test_binary_fourscore() {
        let mut md = FceuMovieData::default();
        md.version = 3;
        md.binary_flag = true;
        md.fourscore = true;
        md.ports = [0, 0, 0];

        let mut rec = MovieRecord::default();
        rec.joysticks = [1, 2, 3, 4];
        rec.commands = 5;
        md.records.push(rec);

        let dumped = dump_fm2(&md, true);
        // Header + "binary 1\n" + "|" + 5 bytes per record
        let loaded = load_fm2(&dumped, false).unwrap();
        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.records[0].commands, 5);
        assert_eq!(loaded.records[0].joysticks, [1, 2, 3, 4]);
    }

    #[test]
    fn test_ffi_load_and_get() {
        let md = make_test_movie();
        let dumped = dump_fm2(&md, false);
        let handle = fceux11_rust_movie_load_fm2(dumped.as_ptr(), dumped.len(), false);
        assert!(!handle.is_null());

        unsafe {
            assert_eq!(fceux11_rust_movie_data_version(handle), 3);
            assert_eq!(fceux11_rust_movie_data_emu_version(handle), 9813);
            assert_eq!(fceux11_rust_movie_data_rerecord_count(handle), 42);
            assert_eq!(fceux11_rust_movie_data_records_count(handle), 2);

            let mut rec = FceuMovieRecord::default();
            assert!(fceux11_rust_movie_data_record_get(handle, 0, &mut rec));
            assert_eq!(rec.joysticks[0], 0b10010001);

            let name = CStr::from_ptr(fceux11_rust_movie_data_rom_filename(handle)).to_string_lossy();
            assert_eq!(name, "test.nes");

            fceux11_rust_movie_data_free(handle);
        }
    }

    #[test]
    fn test_ffi_dump() {
        let rom_filename = std::ffi::CString::new("test.nes").unwrap();
        let guid = std::ffi::CString::new("test-guid").unwrap();

        let mut input = FceuMovieDataInput {
            version: 3,
            emu_version: 9813,
            fds: 0,
            pal_flag: false,
            ppu_flag: false,
            rom_checksum: [0u8; 16],
            rom_filename: rom_filename.as_ptr(),
            savestate: std::ptr::null(),
            savestate_len: 0,
            saveram: std::ptr::null(),
            saveram_len: 0,
            records: std::ptr::null(),
            records_count: 0,
            comments: std::ptr::null(),
            comments_count: 0,
            subtitles: std::ptr::null(),
            subtitles_count: 0,
            rerecord_count: 0,
            guid: guid.as_ptr(),
            binary_flag: false,
            load_frame_count: -1,
            ports: [0, 0, 0],
            fourscore: false,
            microphone: false,
            ram_init_option: 0,
            ram_init_seed: 0,
        };

        let rec = FceuMovieRecord {
            joysticks: [0x81, 0, 0, 0], // R......A
            ..Default::default()
        };
        input.records = &rec;
        input.records_count = 1;

        let out_handle = crate::emufile::fceux11_rust_emufile_mem_create();
        assert!(!out_handle.is_null());

        unsafe {
            let mut curr_frame_pos = -1i32;
            let bytes = fceux11_rust_movie_data_dump(
                &input,
                out_handle,
                false,
                false,
                0,
                &mut curr_frame_pos,
            );
            assert!(bytes > 0);

            // seek back to start before reading
            crate::emufile::fceux11_rust_emufile_mem_fseek(out_handle, 0, 0);

            let size = crate::emufile::fceux11_rust_emufile_mem_size(out_handle) as usize;
            let mut buf = vec![0u8; size];
            crate::emufile::fceux11_rust_emufile_mem_fread(
                out_handle,
                buf.as_mut_ptr(),
                size,
            );

            let s = String::from_utf8(buf).unwrap();
            assert!(s.contains("version 3\n"));
            assert!(s.contains("romFilename test.nes\n"));
            // ports=[0,0,0] -> both gamepads; joy1 defaults to 0 -> "........"
            assert!(s.contains("|0|R......A|........||\n"));

            crate::emufile::fceux11_rust_emufile_mem_destroy(out_handle);
        }
    }
}


