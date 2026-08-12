# Stable layout contract

`CheetahString` keeps three storage modes in a private Rust enum:

| Mode | Payload | Ownership |
|---|---|---|
| Inline | constrained length plus 23 UTF-8 bytes | stored in the value |
| Static | `&'static str` | borrowed forever |
| Shared | `Arc<str>` | immutable reference-counted ownership |

The inline length is a `repr(u8)` enum with exactly 24 valid values, from 0
through 23. Its other bit patterns are invalid discriminants. Rust can use those
invalid patterns as niches for the outer storage variants and for `Option`, so
the largest 24-byte payload does not require a separate discriminant byte.

This design preserves pointer provenance. It does not cast pointers to integers,
reconstruct pointers, use a union, implement manual drop logic, or add an unsafe
block. Static and shared values remain ordinary references and `Arc<str>` values.

## Enforced sizes

The supported 32-bit and 64-bit targets enforce:

| Type | Size |
|---|---:|
| `CheetahString` | 24 bytes |
| `Option<CheetahString>` | 24 bytes |
| 10,000 `CheetahString` vector slots | 240,000 bytes |
| 10,000 `(CheetahString, u64)` map-entry payloads on 64-bit | 320,000 bytes |

The previous 64-bit representation used 320,000 bytes for 10,000 vector slots;
the compact representation saves 80,000 bytes, or 25%, before allocator
overhead. Its vector slot size now matches `String`, while long clones retain
the O(1), zero-allocation `Arc<str>` behavior that `String` does not provide.

Rust enum layout is not a public ABI guarantee. The project therefore treats
24 bytes as a tested performance contract rather than an FFI promise. The CI
matrix checks stable and nightly toolchains, the Rust 1.95 packaged consumer,
and a 32-bit target. Any compiler that stops applying the required niche
optimization fails the layout gate instead of silently changing the footprint.

## Verification

```bash
cargo test --test layout_snapshot --all-features -- --nocapture
cargo test --test allocation_contract --all-features -- --test-threads=1
cargo bench --bench shared_backing -- __allocation_evidence_only__ --noplot \
  2>&1 | tee target/allocation-evidence.log
python scripts/verify-allocation-evidence.py target/allocation-evidence.log
```

Miri remains the behavioral provenance check for the stable representation.
The layout snapshot and schema-v3 benchmark evidence independently enforce the
object size, container slot footprint, allocation counts, and shared-pointer
retention contract.
