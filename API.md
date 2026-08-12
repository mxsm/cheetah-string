# API compatibility

CheetahString 3.1 keeps the stable 3.x source surface while tightening the
private implementation. Public stable methods, traits, variants, aliases, and
return types are not removed or renamed in this release line.

There is one documented safety exception: the published
`experimental-packed` module was removed after strict-provenance Miri exposed
undefined behavior in its safe API. The Cargo feature name remains as a no-op
for dependency resolution, but code importing `cheetah_string::packed` must
migrate to `CheetahString` or `CheetahBuilder`. A registry comparison against
3.0.0 therefore reports the expected removal of the experimental module, its
constant, and `PackedCheetahString`.

## Pattern contract

`starts_with`, `ends_with`, and `contains` accept the sealed `StrPattern` types:

| Pattern | Meaning |
|---|---|
| `char` | one Unicode scalar value |
| `&str` | borrowed string pattern |
| `&String` | borrowed owned string pattern |

External `StrPattern` implementations are rejected by the sealed supertrait.
Query methods classify supported values through a private enum. The hidden
`StrPattern::as_str_pattern` method and its dispatch value remain callable only
as 3.1 compatibility surfaces; the query implementation does not depend on
them.

`split_char` exposes a double-ended standard iterator. `split_str` remains
forward-only because standard string-pattern splitting cannot guarantee reverse
iteration. These capabilities are expressed by the return types rather than by
a runtime panic path.

## Error contract

The operations intentionally return the most precise existing error type:

| Operation | Error type | Buffer recovery |
|---|---|---|
| `try_from_bytes`, `try_from_vec`, `try_from_arc_vec` | `core::str::Utf8Error` | No |
| `try_from_bytes_buf` | `core::str::Utf8Error` | No |
| `try_copy_from_bytes` | `FromUtf8BytesError` | Yes, via `into_bytes` or `into_parts` |
| `CheetahBytes::try_into_string` | `core::str::Utf8Error` | No |
| `try_substring` | `cheetah_string::Error` through `Result` | Not applicable |

`Error::Utf8Error` and `From<Utf8Error>` remain available for existing callers
that aggregate errors into the crate's compatibility error type. UTF-8
constructors continue to return `Utf8Error` directly; 3.1 does not rewrite their
signatures. Range variants remain exhaustive and unchanged for existing match
expressions.

## Automated compatibility gate

`.github/workflows/api-compatibility.yml` runs `cargo-semver-checks` with
minor-release rules against `origin/main` and all features. This protects each
new pull request after the packed retirement baseline; it does not conceal the
approved experimental safety removal relative to the published 3.0.0 crate.
The repository also compiles a downstream-style test that calls the hidden
compatibility method and checks the exact constructor and substring result
types.

Local reproduction:

```bash
cargo semver-checks check-release \
  --baseline-rev origin/main \
  --all-features \
  --release-type minor
cargo test --test api_contract --all-features
```
