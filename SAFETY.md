# Safety model

This file records the unsafe boundaries that remain in the stable
`CheetahString` 3.1 implementation. The retired packed representation is not
part of this model and is not exported by the crate.

## Core invariants

- Every `CheetahString` is valid UTF-8 for its complete lifetime.
- Inline length never exceeds the 23-byte inline buffer.
- Static storage contains a valid `&'static str`.
- Shared storage is an owned `Arc<str>` and preserves pointer provenance.
- `CheetahBytes` has byte semantics and does not imply UTF-8.

## Unsafe boundary inventory

| Boundary | Caller obligation | Internal proof |
|---|---|---|
| `CheetahString::from_utf8_unchecked_vec` | The complete vector is valid UTF-8. | The private unsafe helper either copies at most 23 validated bytes inline or converts the owned vector to `String` before freezing it. |
| `CheetahString::from_utf8_unchecked_bytes` | The complete slice is valid UTF-8. | The validated `str` view is immediately copied into canonical storage. |
| `CheetahString::from_utf8_unchecked_arc_vec` | The complete shared vector is valid UTF-8. | Unique input follows the vector boundary; shared input is copied through the validated slice boundary. |
| `CheetahString::from_utf8_unchecked_bytes_buf` | The complete `bytes::Bytes` payload is valid UTF-8. | The payload is copied through the validated slice boundary. |
| `CheetahBytes::into_string_unchecked` | The complete byte payload is valid UTF-8. | The obligation is forwarded to the `bytes::Bytes` text boundary. |
| SSE2 equality helper | SSE2 is available and both slices have equal length. | Runtime/target gating establishes SSE2, and each unaligned 16-byte load is guarded by the loop bounds. |

The private helpers whose names contain `validated` are themselves `unsafe fn`.
Safe constructors perform UTF-8 validation before entering those helpers;
public unchecked constructors forward their documented caller contract in an
explicit unsafe block.

## Verification

Run the stable representation under Miri:

```bash
cargo +nightly miri test --lib --no-default-features
cargo +nightly miri test --test basic --no-default-features
cargo +nightly miri test --test bytes --features bytes
```

Compile all fuzz targets with libFuzzer and AddressSanitizer:

```bash
cargo +nightly fuzz build
```

The same commands are encoded in `.github/workflows/safety.yml` with pinned
third-party actions.
