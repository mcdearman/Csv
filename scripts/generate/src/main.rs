//! Writes `src/Cases.mw` for csv.
//!
//! ```text
//! cargo run --release -- <package root>
//! ```
//!
//! Random CSV input read with random configurations and random sequences of
//! reads, header reads and header settings; random writes with random
//! configurations; and strings the writer may or may not take for numbers —
//! with what the crate makes of them. The library is ported by hand into
//! `src/`, and the sources of `csv` and `csv-core` are fingerprinted.

use csv::{
    ByteRecord, QuoteStyle, Reader, ReaderBuilder, StringRecord, Terminator, Trim, WriterBuilder,
};
use std::fmt::Write as _;
use std::path::PathBuf;

/// The crate version pinned in `Cargo.toml`.
const UPSTREAM_VERSION: &str = "1.4.0";

/// The fingerprint of the sources `src/` ports.
const SOURCES: u64 = 0x5272_d665_ce8e_1d09;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "../..".into()));

    let print = fingerprint(include_str!(concat!(env!("OUT_DIR"), "/sources.rs.txt")));
    if print != SOURCES {
        eprintln!(
            "error: csv is not the version src/ ports.\n\
             Compare its source in {} with the previous version, carry any change\n\
             into src/, then set SOURCES in scripts/generate/src/main.rs to\n\
             {print:#x}",
            env!("UPSTREAM_DIR")
        );
        std::process::exit(1);
    }

    let cases = cases();
    let path = root.join("src/Cases.mw");
    std::fs::write(&path, &cases).unwrap();
    eprintln!("wrote {} ({} bytes)", path.display(), cases.len());
}

/// FNV-1a: stable across builds, which `DefaultHasher` does not promise.
fn fingerprint(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

// --- encoding -----------------------------------------------------------------------

/// A number as `digits` base-64 digits, most significant first, each digit the
/// character `'0' + d`: `'0'` to `'o'`, one contiguous run of ASCII.
fn digits(out: &mut String, value: u64, digits: u32) {
    assert!(
        value < 1 << (6 * digits),
        "{value} does not fit in {digits} digits"
    );
    for k in (0..digits).rev() {
        out.push(char::from(b'0' + ((value >> (6 * k)) & 63) as u8));
    }
}

/// A string, as its length in bytes (3 digits) and then its bytes.
fn text(out: &mut String, s: &str) {
    digits(out, s.len() as u64, 3);
    out.push_str(s);
}

/// Bytes, which need not be UTF-8: the count (3 digits), then each byte as 2
/// digits.
fn bytes(out: &mut String, b: &[u8]) {
    digits(out, b.len() as u64, 3);
    for &x in b {
        digits(out, x.into(), 2);
    }
}

fn flag(out: &mut String, b: bool) {
    digits(out, u64::from(b), 1);
}

fn maybe_byte(out: &mut String, b: Option<u8>) {
    flag(out, b.is_some());
    if let Some(b) = b {
        digits(out, b.into(), 2);
    }
}

/// `text` as one Meadow string literal, broken with `\`-newline every `width`
/// characters. Printable ASCII is written raw, and everything else escaped;
/// a space that would start a line is `\x20`, since a continuation drops
/// leading whitespace.
fn long_literal(text: &str, width: usize) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / width * 4 + 2);
    out.push('"');
    for (i, c) in text.chars().enumerate() {
        let line_start = i > 0 && i % width == 0;
        if line_start {
            out.push_str("\\\n    ");
        }
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            ' ' if line_start => out.push_str("\\x20"),
            ' '..='~' => out.push(c),
            _ => {
                let _ = write!(out, "\\u{{{:X}}}", u32::from(c));
            }
        }
    }
    out.push('"');
    out
}

// --- inputs -------------------------------------------------------------------------

/// A small deterministic generator, so that the cases are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
}

const PIECES: &[&[u8]] = &[
    b"a",
    b"bc",
    "日本".as_bytes(),
    "é".as_bytes(),
    b" x ",
    "\u{3000}y\u{a0}".as_bytes(),
    b"1.5",
    b"\"",
    b"\"",
    b"'",
    b"\\",
    b"#",
    b",",
    b",",
    b";",
    b"\t",
    b"|",
    b"\r",
    b"\n",
    b"\n",
    b"\r\n",
    b"\x1f",
    b"\x1e",
    b"x",
    b"\xff",
    b"\xc3",
    b"\xed\xa0\x80",
    b"  ",
    b"\xef\xbb\xbf",
];

const HEADER_TEXTS: &[&str] = &[" h1 ", "日", "", "name", "\u{3000}wide"];

const HEADER_BYTES: &[&[u8]] = &[b" h1 ", b"\xff", b"", b"id", b"\t"];

fn random_input(rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::new();
    if rng.chance(10) {
        v.extend_from_slice(b"\xef\xbb\xbf");
    }
    for _ in 0..rng.below(40) {
        v.extend_from_slice(rng.pick(PIECES));
    }
    v
}

fn string_record(out: &mut String, r: &StringRecord) {
    digits(out, r.len() as u64, 2);
    for f in r {
        bytes(out, f.as_bytes());
    }
    position(out, r.position());
}

fn byte_record(out: &mut String, r: &ByteRecord) {
    digits(out, r.len() as u64, 2);
    for f in r {
        bytes(out, f);
    }
    position(out, r.position());
}

fn position(out: &mut String, p: Option<&csv::Position>) {
    flag(out, p.is_some());
    if let Some(p) = p {
        digits(out, p.byte(), 3);
        digits(out, p.line(), 3);
        digits(out, p.record(), 3);
    }
}

fn error(out: &mut String, e: &csv::Error) {
    digits(out, 2, 1);
    text(out, &e.to_string());
}

/// One reading case: the configuration, the input, the operations, and
/// what each gave.
fn reader_case(rng: &mut Rng, out: &mut String) {
    let mut b = ReaderBuilder::new();
    let delimiter = rng.pick(b",;\t|a\"");
    digits(out, delimiter.into(), 2);
    b.delimiter(delimiter);
    let crlf = rng.chance(50);
    flag(out, crlf);
    if crlf {
        b.terminator(Terminator::CRLF);
    } else {
        let t = rng.pick(b"\n;x\r|");
        digits(out, t.into(), 2);
        b.terminator(Terminator::Any(t));
    }
    let quote = rng.pick(b"\"'|");
    digits(out, quote.into(), 2);
    b.quote(quote);
    let escape = rng.chance(40).then(|| rng.pick(b"\\\"#"));
    maybe_byte(out, escape);
    b.escape(escape);
    let double_quote = rng.chance(70);
    let quoting = rng.chance(85);
    flag(out, double_quote);
    flag(out, quoting);
    b.double_quote(double_quote).quoting(quoting);
    let comment = rng.chance(40).then(|| rng.pick(b"#\"x"));
    maybe_byte(out, comment);
    b.comment(comment);
    let flexible = rng.chance(40);
    let headers = rng.chance(60);
    flag(out, flexible);
    flag(out, headers);
    b.flexible(flexible).has_headers(headers);
    let trim = rng.below(4);
    digits(out, trim, 1);
    b.trim([Trim::None, Trim::Headers, Trim::Fields, Trim::All][trim as usize]);
    let capacity = if rng.chance(30) {
        8192
    } else {
        1 + rng.below(16)
    };
    digits(out, capacity, 3);
    b.buffer_capacity(capacity as usize);
    let ascii = rng.chance(5);
    flag(out, ascii);
    if ascii {
        b.ascii();
    }

    let input = random_input(rng);
    bytes(out, &input);
    let mut rdr: Reader<&[u8]> = b.from_reader(&input[..]);
    let mut rec = StringRecord::new();
    let mut brec = ByteRecord::new();
    let ops = rng.below(10);
    digits(out, ops, 1);
    for _ in 0..ops {
        let op = rng.below(8);
        digits(out, op, 1);
        match op {
            0 => match rdr.read_record(&mut rec) {
                Ok(false) => digits(out, 0, 1),
                Ok(true) => {
                    digits(out, 1, 1);
                    string_record(out, &rec);
                }
                Err(e) => error(out, &e),
            },
            1 => match rdr.read_byte_record(&mut brec) {
                Ok(false) => digits(out, 0, 1),
                Ok(true) => {
                    digits(out, 1, 1);
                    byte_record(out, &brec);
                }
                Err(e) => error(out, &e),
            },
            2 => match rdr.headers() {
                Ok(h) => {
                    digits(out, 1, 1);
                    string_record(out, h);
                }
                Err(e) => error(out, &e),
            },
            3 => match rdr.byte_headers() {
                Ok(h) => {
                    digits(out, 1, 1);
                    byte_record(out, h);
                }
                Err(e) => error(out, &e),
            },
            4 => {
                let n = rng.below(4);
                digits(out, n, 1);
                let fields: Vec<&str> = (0..n).map(|_| rng.pick(HEADER_TEXTS)).collect();
                for f in &fields {
                    bytes(out, f.as_bytes());
                }
                rdr.set_headers(StringRecord::from(fields));
            }
            5 => {
                let n = rng.below(4);
                digits(out, n, 1);
                let fields: Vec<&[u8]> = (0..n).map(|_| rng.pick(HEADER_BYTES)).collect();
                for f in &fields {
                    bytes(out, f);
                }
                rdr.set_byte_headers(ByteRecord::from(fields));
            }
            6 => {
                let results: Vec<_> = rdr.records().collect();
                digits(out, results.len() as u64, 2);
                for r in results {
                    match r {
                        Ok(r) => {
                            digits(out, 1, 1);
                            string_record(out, &r);
                        }
                        Err(e) => error(out, &e),
                    }
                }
            }
            _ => {
                let results: Vec<_> = rdr.byte_records().collect();
                digits(out, results.len() as u64, 2);
                for r in results {
                    match r {
                        Ok(r) => {
                            digits(out, 1, 1);
                            byte_record(out, &r);
                        }
                        Err(e) => error(out, &e),
                    }
                }
            }
        }
    }
    position(out, Some(rdr.position()));
    flag(out, rdr.is_done());
}

const FIELDS: &[&[u8]] = &[
    b"",
    b"",
    b"a",
    b"b,c",
    b"q\"uote",
    b"it's",
    b"new\nline",
    b"cr\r",
    b"#hash",
    b"1.5",
    b"-3",
    b"inf",
    b"NaN",
    b"1e5",
    b".",
    "日本".as_bytes(),
    b"\xff",
    b"x;y",
    b"tab\t",
    b"esc\\",
    b"!",
    b"a|b",
    b"\"\"",
];

/// One writing case: the configuration, the operations and their results,
/// and what was written.
fn writer_case(rng: &mut Rng, out: &mut String) {
    let mut b = WriterBuilder::new();
    let delimiter = rng.pick(b",;\tx");
    digits(out, delimiter.into(), 2);
    b.delimiter(delimiter);
    let term = rng.below(5);
    digits(out, term, 1);
    b.terminator(match term {
        0 => Terminator::CRLF,
        1 => Terminator::Any(b'\n'),
        2 => Terminator::Any(b'\r'),
        3 => Terminator::Any(b';'),
        _ => Terminator::Any(b'|'),
    });
    let style = rng.below(4);
    digits(out, style, 1);
    b.quote_style(
        [
            QuoteStyle::Always,
            QuoteStyle::Necessary,
            QuoteStyle::NonNumeric,
            QuoteStyle::Never,
        ][style as usize],
    );
    let quote = rng.pick(b"\"'");
    let escape = rng.pick(b"\\\"!");
    digits(out, quote.into(), 2);
    digits(out, escape.into(), 2);
    b.quote(quote).escape(escape);
    let double_quote = rng.chance(60);
    flag(out, double_quote);
    b.double_quote(double_quote);
    let comment = rng.chance(30).then_some(b'#');
    maybe_byte(out, comment);
    b.comment(comment);
    let flexible = rng.chance(40);
    flag(out, flexible);
    b.flexible(flexible);
    let capacity = if rng.chance(30) {
        8192
    } else {
        2 + rng.below(30)
    };
    digits(out, capacity, 3);
    b.buffer_capacity(capacity as usize);

    let mut wtr = b.from_writer(Vec::new());
    let ops = rng.below(15);
    digits(out, ops, 1);
    for _ in 0..ops {
        let op = rng.below(4);
        digits(out, op, 1);
        let result = match op {
            0 => {
                let f = rng.pick(FIELDS);
                bytes(out, f);
                wtr.write_field(f)
            }
            1 | 2 => {
                let n = rng.below(4);
                digits(out, n, 1);
                let fields: Vec<&[u8]> = (0..n).map(|_| rng.pick(FIELDS)).collect();
                for f in &fields {
                    bytes(out, f);
                }
                if op == 1 {
                    wtr.write_record(&fields)
                } else {
                    wtr.write_byte_record(&ByteRecord::from(fields))
                }
            }
            _ => wtr.flush().map_err(csv::Error::from),
        };
        match result {
            Ok(()) => flag(out, true),
            Err(e) => {
                flag(out, false);
                text(out, &e.to_string());
            }
        }
    }
    let written = wtr.into_inner().unwrap();
    bytes(out, &written);
}

const NUMBERS: &[&str] = &[
    "",
    "0",
    "-0",
    "+5",
    "1.",
    ".5",
    ".",
    "e5",
    "1e",
    "1e+",
    "1e-5",
    "1E5",
    "+.5e-3",
    "inf",
    "-inf",
    "+Infinity",
    "INFINITY",
    "infin",
    "nan",
    "NaN",
    "-nan",
    "1_000",
    " 1",
    "1 ",
    "0x10",
    "1.5.5",
    "--1",
    "+",
    "-",
    "12345678901234567890123456789012345678901234567890",
    "1e999",
    "٣",
    "1.e5",
    "e",
    "E",
    "5e5e5",
];

// --- cases --------------------------------------------------------------------------

fn cases() -> String {
    let mut rng = Rng(0xc5f0_ea11_d0c5_0001);
    let reader_count = 1500;
    let mut readers = String::new();
    for _ in 0..reader_count {
        reader_case(&mut rng, &mut readers);
    }
    let writer_count = 1500;
    let mut writers = String::new();
    for _ in 0..writer_count {
        writer_case(&mut rng, &mut writers);
    }
    let mut numbers = String::new();
    let mut number_count = 0;
    let mut add_number = |s: &str, out: &mut String| {
        text(out, s);
        flag(out, csv_core::is_non_numeric(s.as_bytes()));
        number_count += 1;
    };
    for s in NUMBERS {
        add_number(s, &mut numbers);
    }
    let alphabet: &[u8] = b"0123456789.eE+-infaINFANx ";
    for _ in 0..500 {
        let n = rng.below(7) as usize;
        let s: String = (0..n).map(|_| char::from(rng.pick(alphabet))).collect();
        add_number(&s, &mut numbers);
    }

    let mut out = String::new();
    let _ = writeln!(
        out,
        "-- GENERATED by scripts/generate.sh from csv {UPSTREAM_VERSION}.
-- Do not edit: run the script again instead.
--
-- Inputs, with what the crate makes of them, for `Tests.mw`: {reader_count}
-- readings, {writer_count} writings and {number_count} strings that may be numbers.
--
-- Copyright Andrew Gallant and the rust-csv contributors, and the Meadow
-- port's authors. Licensed under MIT or the Unlicense: see LICENSE-MIT,
-- UNLICENSE and COPYRIGHT.
--
-- A number is base-64 digits; a string is its length in bytes (3 digits) and
-- then its bytes; bytes are their count (3 digits) and then 2 digits each.

-- Each reading is its configuration, its input, and its operations with what
-- each gave, in the order `Tests.mw` reads them; then where the reader ended.
@cfg(test)
@pub(pkg) def readings =
  {}

-- Each writing is its configuration, its operations with whether each
-- succeeded, and what was written.
@cfg(test)
@pub(pkg) def writings =
  {}

-- Each string, and whether the writer takes it for something other than a
-- number.
@cfg(test)
@pub(pkg) def numbers =
  {}",
        long_literal(&readers, 96),
        long_literal(&writers, 96),
        long_literal(&numbers, 96)
    );
    out
}
