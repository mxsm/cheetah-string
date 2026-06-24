# SIMD Portability Strategy

## Current Decision

The `simd` feature remains an x86_64 SSE2-only optimization path for now.
It is compiled only behind `#[cfg(all(feature = "simd", target_arch = "x86_64"))]`.

The accelerated surface is intentionally limited to selected byte comparisons:

- equality
- `starts_with`
- `ends_with`

Substring search remains on the `memchr`/`memmem` backend. The project should
not claim SIMD acceleration for `find` or `contains` unless a dedicated search
implementation is wired into those APIs and benchmarked against `memchr`.

## AArch64 / NEON Position

No AArch64 NEON implementation is being added in this phase. The repository has
no current ARM benchmark evidence, and adding a second architecture-specific
unsafe/vectorized path without target hardware evidence would increase
maintenance risk more than it would improve the supported API.

Before adding NEON support, require:

- an AArch64 CI or local benchmark target
- Criterion comparisons against the scalar and `memchr`/`memmem` paths
- feature-gated implementation isolated from the x86_64 SSE2 module
- tests covering the same equality, prefix, and suffix behavior as SSE2

Until those gates exist, the portable behavior is scalar plus `memchr`/`memmem`,
with SSE2 as an optional x86_64 fast path.
