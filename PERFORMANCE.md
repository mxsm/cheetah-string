# Performance contracts

CheetahString separates deterministic performance contracts from timing
measurements. Allocation counts and object layout are merge/release gates;
Criterion timing and RSS observations are diagnostic because hosted runner
noise cannot establish a portable latency threshold.

## Enforced 64-bit contracts

| Operation | Maximum allocation events | Additional invariant |
|---|---:|---|
| Inline construction | 0 | UTF-8 length is at most 23 bytes |
| Static construction | 0 | Explicit static input remains borrowed |
| Long `Arc<str>` adoption | 0 | Payload pointer is retained |
| `char` construction | 0 | Every Unicode scalar remains inline |
| Concatenation with a result of at most 23 bytes | 0 | Result remains inline |
| Long shared clone | 0 | Payload pointer is shared |
| Long borrowed construction | 1 | One live `Arc<str>` backing |
| Exact-capacity owned/builder freeze | 1 | One live `Arc<str>` backing |
| Spare-capacity owned/builder freeze | 2 | Shrink/reallocation plus `Arc<str>` backing |
| `CheetahString` object size | N/A | 24 bytes on supported 32-bit and 64-bit targets |
| `Option<CheetahString>` object size | N/A | 24 bytes on supported 32-bit and 64-bit targets |
| 10,000 vector element slots | N/A | 240,000 bytes; 80,000 below the prior contract |
| 10,000 `(CheetahString, u64)` payloads | N/A | 320,000 bytes; equal to `(String, u64)` |

The allocation count includes allocation and reallocation events during the
measured conversion. It is intentionally different from the number of live
allocations retained by the result.

## Reproducing the gates

Run the direct contracts:

```bash
cargo test --test allocation_contract --all-features -- --test-threads=1
cargo test --test layout_snapshot --all-features -- --nocapture
```

Capture and validate the independent benchmark evidence:

```bash
cargo bench --bench shared_backing -- __allocation_evidence_only__ --noplot \
  2>&1 | tee target/allocation-evidence.log
python scripts/verify-allocation-evidence.py target/allocation-evidence.log
```

The verifier requires one schema-v3 `SHARED_BACKING_EVIDENCE` record and fails
closed when a required field is absent, an allocation count regresses, the
64-bit layout or downstream slot footprint changes, or long `Arc<str>` input
does not retain its pointer.

## Timing policy

Criterion groups remain useful for comparing construction, cloning, inline
fast paths, and RocketMQ-shaped map workloads on the same machine. Their raw
timings and RSS samples must be reported with toolchain, CPU, operating system,
feature set, and source revision. They are not used as cross-run release gates.
