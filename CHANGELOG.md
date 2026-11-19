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
- `contains`, `find`, and `rfind` choose their search backend by length.
  The standard library's SIMD `contains` beat a one-shot `memmem` at every
  measured length from 20 bytes; `str::find` and `str::rfind` beat it from
  24 to 127 bytes, and `str::rfind` also beyond.

### Added

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
