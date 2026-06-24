# Experimental Packed Representation Safety Proof

This document applies only to `feature = "experimental-packed"` and the
`packed::PackedCheetahString` prototype. It is not part of the stable
`CheetahString` representation.

## Layout

- Target gate: 64-bit little-endian only.
- Size target: 24 bytes, represented as `[usize; 3]`.
- Inline representation: bytes `0..23` store UTF-8 data, byte `23` stores len.
- Heap representation: word 0 stores `String` pointer, word 1 stores len, word
  2 stores capacity with the high bit set as the heap tag.
- Heap capacity invariant: high capacity bit must be clear before tagging.

## Invariants

- Inline len is always `<= 23`.
- Inline bytes `[..len]` are always valid UTF-8.
- Heap pointer/len/capacity are produced only from `String` raw parts.
- Heap `cap` is restored by masking out the tag bit before `String::from_raw_parts`.
- Drop reconstructs and drops a heap `String` exactly once.
- Clone of heap storage allocates a fresh `String`; it never aliases ownership.
- `as_str` returns bytes owned by `self` and valid for the borrow lifetime.
- `push_str` either appends in inline storage or rebuilds from safe `String`.

## Required Gates Before Stable Adoption

- `cargo test --features experimental-packed`
- `cargo +nightly miri test --features experimental-packed`
- `RUSTFLAGS="-Z sanitizer=address" cargo +nightly test --features experimental-packed`
- `cargo fuzz run fuzz_packed_from_bytes`
- `cargo fuzz run fuzz_packed_push_str`
- layout snapshot comparison against stable `CheetahString`
- MQ workload benchmark comparison against stable `CheetahString`

On Windows PowerShell, this command captures the required packed evidence under
`bench-results/packed-evidence/<timestamp>/`:

```powershell
scripts/verify-packed.ps1 -RunMiri -RunSanitizer -RunFuzz -RunBench
```

## Current Status

The prototype is intentionally not wired into stable `CheetahString`. It is a
contained experiment used to validate layout, drop/clone behavior, and benchmark
potential before any stable representation change is considered.
