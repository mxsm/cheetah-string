# Changelog

## Unreleased

### Performance

- Clone now bumps the shared count and copies all 24 bytes at once. The
  derived implementation moved bytes 1..8 through overlapping stack writes
  that defeated store-to-load forwarding; inline clone-and-drop fell from
  8.7 ns to 1.2 ns and shared clone-and-drop from 15.5 ns to 8.1 ns on an
  i7-11700K.
- Equality compares two inline values as whole structures (their padding is
  always zero) and short-circuits clones of one shared allocation with
  `Arc::ptr_eq`. 16-byte inline equality fell from 5.8 ns to 1.2 ns and
  equality of two 1 KiB clones from 32.8 ns to 0.8 ns.
- Freezing a long `String` or `CheetahBuilder` with spare capacity copies
  directly into the `Arc<str>` backing instead of shrinking first. The
  allocation contract drops from 2 allocation events to 1, and a 64-byte
  freeze fell from 109 ns to 50 ns.
- Inline construction gathers the bytes with overlapping fixed-size loads
  and writes the value once, instead of calling a variable-length
  `memcpy`. Building and dropping an inline value fell from 5.7-9.6 ns to
  2.0-2.6 ns across lengths of 3 to 23 bytes (compact_str: 5.4-5.9 ns).
- Equality (`==` against `CheetahString`, `str`, `String`, `Vec<u8>`, and
  `CheetahText`), `starts_with`, and `ends_with` compare inputs of up to 64
  bytes with overlapping fixed-size loads instead of calling `memcmp`; longer
  inputs still use `memcmp`. At 32 and 64 bytes, equality of two values fell
  from 3.7 and 4.7 ns to 2.4 and 2.7 ns, and `starts_with`/`ends_with` from
  2.9-3.9 ns to 1.5-2.3 ns for 8- to 64-byte patterns. This is the effect the
  `experimental-simd` SSE2 loop had, in portable safe code.
- `CheetahBuilder::new()` and `with_capacity(n)` for `n <= 23` keep their
  contents inline until they outgrow 23 bytes, so building a short key and
  calling `finish()` allocates nothing. A 14-byte `topic@group` key built
  with an exact capacity fell from 33.9 ns to 8.9 ns, and from 83.5 ns to
  8.7 ns when built from `new()`. `capacity()` reports 23 while the contents
  are inline, and a larger `with_capacity` still allocates up front, so
  `into_string()` keeps returning that buffer without copying. Builds that
  start on the heap are about 5% slower because each push checks the state.
- `replace` with a one-byte pattern and a one-byte replacement, such as
  `replace("-", "_")`, copies the text once and swaps the matching bytes on
  the way, instead of scanning it twice and copying it piece by piece. On an
  i7-11700K a 50-byte value fell from 99 ns to 52 ns, a 14-byte inline value
  from 37 ns to 15 ns, and a 1 KiB value from 1,062 ns to 78 ns.
- `split_str` with a separator of 2 to 32 bytes no longer builds a Two-Way
  searcher on every call. It screens 16 positions at a time on the
  separator's first and last bytes, in a loop the compiler turns into vector
  compares, and checks the middle of the few positions that pass without a
  `memcmp` call. After four positions that pass the screen but do not match,
  it hands the rest of the text to `str::find`. On an i7-11700K splitting
  `"a::b::c::d::e"` fell from 36 ns to 26 ns, a 200-byte value without the
  separator from 65 ns to 22 ns, a 32-byte separator from 98 ns to 20 ns, and
  `"aab"` in 200 `a`s from 186 ns to 23 ns. Text built to pass the screen
  without matching, such as `"a_b"` in `aXb` repeated, is about 5% slower.
- Ordering two inline values compares their zero-padded buffers as three
  big-endian words, then their lengths, instead of calling `memcmp`; the
  order is exactly `str`'s. On an i7-11700K comparing two 14-byte keys fell
  from 7.4 ns to 1.9 ns, sorting 1,024 inline keys from 66.1 µs to
  25.3 µs, and 1,024 `BTreeMap` lookups from 118 µs to 31 µs. Sorting long,
  shared keys was about 5% slower in the same benchmark.
- Inline decoding clamps the length instead of carrying a slice bounds check
  and panic path into every inlined `as_str()` call site.
- `SplitStr::next` is now inlinable, and `SplitStr` implements `Clone`,
  `Debug`, and `FusedIterator`.
- `concat`, `+`, `replace`, and `replacen` compute the exact result length
  and write it once: up to 23 bytes inline with no allocation, otherwise
  straight into one `Arc<str>` allocation instead of a `String` that is
  frozen afterwards. Empty patterns, which match at every character
  boundary, take the same path. A `replace` without a match, or with a count
  of zero, reuses the value.
- `to_uppercase` and `to_lowercase` map ASCII input in place: inline
  results allocate nothing, long results copy once, and values that are
  already in the requested case are reused.
- `contains`, `find`, and `rfind` choose their search backend by haystack
  and needle length, using thresholds measured on x86_64 and aarch64 with
  `benches/search_thresholds.rs`. A needle at least as long as the haystack
  is answered by one comparison. `contains` uses the standard library's SIMD
  search for 2- to 32-byte needles once the haystack holds a 16-byte block
  plus the needle. `find` uses `str::find` when
  `16 + 3 * needle <= haystack < 64`, and `rfind` uses `str::rfind` when
  `haystack >= 24 + 2 * needle`; every other search uses a one-shot `memmem`.
  On an i7-11700K the backend `find` picked was 16% slower than the faster
  one on geometric mean and up to 4.3x slower in the worst case; it is now
  within 0.5% and 1.3x (`rfind`: 11% and 3.7x before, under 1% and 1.5x
  after). The `ubuntu-latest` and `ubuntu-24.04-arm` runners stay within 4%
  and 1.8x. A 16-byte needle in a 32-byte haystack fell from 43 ns to 18 ns
  for both. The `Search thresholds` workflow reruns the measurement on both
  targets when `src/search.rs` changes.

### Added

- `CheetahBuilder` implements `fmt::Write`, so `write!` appends to it.
- `cheetah_format!` and `CheetahString::from_fmt(format_args!(..))` format
  straight into a `CheetahString`: output of up to 23 bytes is written to
  inline storage without allocating, a literal with no arguments is borrowed
  as a static value, and longer output spills to one generously sized
  buffer. A 14-byte `topic@group` key took 104 ns through
  `CheetahString::from(format!(..))` and 19 ns through `cheetah_format!`; a
  50-byte key took 143 ns and 93 ns.
- `CheetahText` (`bytes` feature): immutable UTF-8 text backed by
  `bytes::Bytes`. Validating a `Bytes` buffer, `slice`, `try_slice`, and
  `slice_ref` never copy the text, so decoders can return owned keys and
  values that share the receive buffer. Static and shared `CheetahString`
  values convert without copying; equality, ordering, and hashing match
  `str`.
- `CheetahString::concat(&[&str])` builds a value from parts with one copy.
- `find_char` and `rfind_char`, matching the existing `*_char` queries.
- `&CheetahString` is accepted wherever `StrPattern` is, for example
  `value.contains(&other)`.
- `From<&CheetahString> for String`, an exact-capacity copy that skips the
  `Display` path used by `to_string()`.
- `#[must_use]` on query and transform methods, which return new values
  and never modify `self`.

### Deprecated

- The `experimental-simd` feature and its `simd` alias are compatibility
  no-ops. Their x86_64 SSE2 comparison loop, 186 lines with six `unsafe`
  blocks and one `unsafe fn`, is removed; equality, `starts_with`, and
  `ends_with` use the portable comparison in every build. Default builds are
  unchanged. For builds that enabled the feature, measured on an i7-11700K:
  24-byte equality fell from 6.0 ns to 3.1 ns and `starts_with` from 4.9 ns
  to 2.0 ns, while equality of 32, 65, and 256-byte values rose by 0.2 to
  1.1 ns.

### Verification

- Pull requests compare the per-iteration instruction counts of 30 fixed
  scenarios (construction, clone, equality, ordering, hashing, search,
  splitting, building, formatting, and transforms) with the base branch
  under Valgrind's cachegrind, and fail when a scenario runs more than 2%
  and more than 2 instructions slower. The `accept-instruction-regression`
  label reports a deliberate regression without failing.

## 3.1.0

### Security

- Retired the public `experimental-packed` v1 implementation. Its heap path
  converted an allocation pointer to `usize` and reconstructed a pointer later;
  strict-provenance Miri rejected that reconstruction and deallocation. The
  Cargo feature name remains as a deprecated no-op for 3.x dependency
  compatibility, but `packed::PackedCheetahString` is no longer available.
- Reordered release automation so all validation and candidate digest checks
  complete before a tag is created or a package is published.
- Made every internal unchecked UTF-8 conversion an explicit unsafe boundary
  and added Miri coverage for the stable representation.

### Toolchain and verification

- Raised the minimum supported Rust version from 1.75 to 1.95.
- Added reproducible MSRV, rustdoc, dependency-audit, repository-contract,
  Miri, and fuzz-build gates.
- Removed the unmaintained `smartstring` benchmark dependency and retired the
  stale score-verification scripts whose evidence manifest was not shipped.

### Performance and API

- Added `CheetahString::from_arc_str` and `From<Arc<str>>`; long values preserve
  the input payload pointer with zero allocation, while short values remain
  canonical inline strings.
- Removed temporary heap allocation from `From<char>` and inline-sized
  concatenation.
- Exposed the reverse, clone, and fused iterator guarantees of `lines()`.
- Added fail-closed allocation evidence for exact/spare freezes, shared input,
  cloning, characters, and short concatenation.
- Reduced `CheetahString` and `Option<CheetahString>` from 32 to 24 bytes on
  64-bit targets while preserving the 23-byte inline capacity. The stable
  representation uses constrained Rust enum discriminants as layout niches and
  does not encode pointers as integers.
- Moved text-query pattern dispatch behind a private sealed classifier while
  retaining the 3.1 compatibility method and dispatch value unchanged.
- Added downstream error-signature contracts and an automated minor-release
  semver comparison against the main branch.

### Migration

- Replace read-only `PackedCheetahString` values with `CheetahString`.
- Replace append-heavy uses with `CheetahBuilder` or `String`, freezing to
  `CheetahString` only after mutation completes.
- Code that relied on the retired mutable packed API must still migrate its
  mutation flow; the stable immutable value now provides a safe 24-byte layout.

### Compatibility

- This release remains on the 3.x API line. Stable `CheetahString` APIs are not
  removed, and the `experimental-packed` feature continues to resolve as a
  no-op.
