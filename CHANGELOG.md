# Changelog

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
