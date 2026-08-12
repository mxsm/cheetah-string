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

### Migration

- Replace read-only `PackedCheetahString` values with `CheetahString`.
- Replace append-heavy uses with `CheetahBuilder` or `String`, freezing to
  `CheetahString` only after mutation completes.
- Code that relied on a 24-byte object has no safe drop-in replacement and
  must re-evaluate its container-memory requirements.

### Compatibility

- This release remains on the 3.x API line. Stable `CheetahString` APIs are not
  removed, and the `experimental-packed` feature continues to resolve as a
  no-op.
