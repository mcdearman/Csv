# csv

CSV reading and writing, for [Meadow](https://github.com/mcdearman/meadow).
It reads quoted fields, escapes, comments, headers, any delimiter and record
terminator, and text that is not UTF-8. It writes fields with the quoting each
one needs.

This package is a port of Rust's [`csv`](https://github.com/BurntSushi/rust-csv)
1.4.0, together with the `csv-core` 0.1.13 parser it is built on. It reads
and writes exactly as the crates do, including record positions and error
messages. Serde support is not ported: records are lists of fields.

## Install

```sh
meadow add mcdearman/meadow-csv
```

## Use

```meadow
use csv
use Std.Collections.Vector as V

def input = "city;population\nOslo;709037\n\"Den Haag\";552995\n"

def main =
  match readAll input (readerBuilder |> withDelimiter ';') with
  | Ok rows ->
      writeAll (V.map (\(r : StringRecord) -> V.reverse r.fields) rows) (writerBuilder |> withQuoteStyle Always)
  | Err e -> Err e
```

This gives:

```text
Ok("\"709037\",\"Oslo\"\n\"552995\",\"Den Haag\"\n")
```

The crate's readers and writers change in place. Here each operation returns
a new reader or writer alongside its result. The configurations are records,
changed with `with` functions that take the configuration last so that calls
chain with `|>`.

### Reading

- **Configure:** start from `readerBuilder` and change it with:
  - `withDelimiter`, `withQuote`, `withEscape` and `withComment`, which take
    ASCII characters;
  - `withTerminator` (`CRLF` or `Any byte`);
  - `withHasHeaders`;
  - `withFlexible`, which allows records of different lengths;
  - `withTrim` (`NoTrim`, `Headers`, `Fields` or `All`);
  - `withDoubleQuote`, `withQuoting`, `withAscii` and `withBufferCapacity`.
- **Open** a reader with `fromString text builder` or
  `fromBytes bytes builder`.
- **Read** with:
  - `readRecord r`, which gives `(Ok (Just record), r')` for a record,
    `(Ok None, r')` at the end, or an error;
  - `readByteRecord r`, the same with fields as bytes;
  - `records r` and `byteRecords r`, which read to the end and give each
    record or error;
  - `headers r` and `byteHeaders r`;
  - `setHeaders fields r` and `setByteHeaders fields r`.
- **Inspect** with `position r`, `isDone r` and `hasHeaders r`.
- **Shortcut:** `readAll text builder` gives every record, or the first error.

A `StringRecord` has `fields : [String]`, and a `ByteRecord` has
`fields : [#[UInt8]]`. Both have a `position`, with `byte`, `line` and
`index` (the crate's `record`). A `CsvError` is `Utf8` or `UnequalLengths`.
`errorMessage` gives its text as the crate displays it, and `errorPosition`
gives where it happened.

### Writing

- **Configure:** start from `writerBuilder` and change it with the shared
  `with` functions, `withQuoteStyle` (`Always`, `Necessary`, `NonNumeric` or
  `Never`) and `withWriterEscape`.
- **Create** a writer with `writer builder`.
- **Write** with:
  - `writeRecord fields w`, `writeRecordBytes fields w` and
    `writeByteRecord fields w`, which give `(Ok (), w')` or an error;
  - `writeField text w` and `writeByteField bytes w`.
- **Finish** with `toText w` or `toBytes w`. `flush w` empties the buffer.
- **Shortcut:** `writeAll rows builder` gives the text, or the first error.

`isNonNumeric bytes` is the test `NonNumeric` quoting uses.

### Where it follows the crate closely

- **Line numbers.** The crate counts a line feed only when its parser steps
  onto it, not when it copies a run of ordinary bytes. When a line feed is
  not a terminator, the count therefore depends on:
  - the size of the reading buffer;
  - how the reader's record buffers have grown.

  The reader keeps both as the crate does, so positions match exactly.
- **The fast path.** `writeByteRecord` copies a record straight into the
  buffer when it is sure to fit, as the crate does. That path skips the
  bookkeeping of `writeField`, so mixing the two gives the crate's output.
- **Other quirks** are kept:
  - a comment at the end of the input, without a line feed, reads as an
    empty record;
  - a record that fails its length check still counts;
  - a byte order mark is skipped only in the reader's first buffer.

## How it's made

The modules in `src/` translate `csv-core`'s parser and writer and `csv`'s
reader, writer and records. The parser's DFA is built from its NFA, as the
crate builds it.

**`src/Cases.mw`** holds three kinds of checks:

- **1,500 random readings.** Each has a random configuration and input, then
  a random mix of reads, header reads, header settings and full iterations.
  The inputs include:
  - quotes, escapes and comments;
  - every terminator;
  - byte order marks;
  - invalid UTF-8;
  - small buffers.
- **1,500 random writings.** Each has a random configuration and mix of field,
  record and byte-record writes, with small buffers.
- **536 strings** that the writer may or may not take for numbers.

Run `scripts/generate.sh` to regenerate; it needs a Rust toolchain.

## Licence

MIT or the Unlicense, like csv: see [LICENSE-MIT](LICENSE-MIT),
[UNLICENSE](UNLICENSE), [COPYING](COPYING) and [COPYRIGHT](COPYRIGHT).
