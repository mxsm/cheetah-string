# Benchmark Artifacts

This directory defines the artifact layout for performance-sensitive changes.
Generated benchmark output should be committed only when it is intentionally
used as review evidence for a release or PR.

Recommended layout:

```text
bench-results/
  layout/
    current.json
    v1.1.json
    v1.2.json
    v2-packed.json
  criterion/
    before/
    after/
  mq/
    topic.json
    properties.json
    remoting-header.json
  packed-evidence/
    YYYYMMDD-HHMMSS/
      summary.md
      packed-test.txt
      packed-miri.txt
      packed-asan.txt
      fuzz-packed-from-bytes.txt
      fuzz-packed-push-str.txt
      packed-bench.txt
  summaries/
    summary-v1.1-v1.2.md
    summary-v1.2-v2-packed.md
```

Minimum metadata for generated JSON artifacts:

```json
{
  "crate": "cheetah-string",
  "version": "2.0.0",
  "profile": "release",
  "target": "x86_64-unknown-linux-gnu",
  "rustc": "rustc 1.xx.x",
  "os": "linux",
  "cpu": "model name",
  "bench": "layout"
}
```

For local capture, run:

```bash
scripts/bench-all.sh current
```

On Windows PowerShell:

```powershell
scripts/bench-all.ps1 current
```

For the experimental packed representation evidence gate:

```powershell
scripts/verify-packed.ps1 -RunMiri -RunSanitizer -RunFuzz -RunBench
```

The packed evidence script always runs `cargo test --features experimental-packed`.
The Miri, sanitizer, fuzz, and packed benchmark gates are opt-in because they
require nightly components, optional cargo subcommands, target support, or more
runtime than the default test suite.
