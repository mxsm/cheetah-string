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
  summaries/
    summary-v1.1-v1.2.md
    summary-v1.2-v2-packed.md
```

Minimum metadata for generated JSON artifacts:

```json
{
  "crate": "cheetah-string",
  "version": "1.2.0",
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
