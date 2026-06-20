$Version = if ($args.Count -gt 0) { $args[0] } else { "current" }
$ResultDir = Join-Path "bench-results" $Version

New-Item -ItemType Directory -Force -Path $ResultDir | Out-Null

cargo test layout_snapshot --all-features -- --nocapture |
    Tee-Object -FilePath (Join-Path $ResultDir "layout-test.txt")

cargo bench --bench layout |
    Tee-Object -FilePath (Join-Path $ResultDir "layout-bench.txt")

cargo bench --bench comprehensive |
    Tee-Object -FilePath (Join-Path $ResultDir "comprehensive.txt")

cargo bench --bench pattern |
    Tee-Object -FilePath (Join-Path $ResultDir "pattern.txt")

cargo bench --bench simd --features simd |
    Tee-Object -FilePath (Join-Path $ResultDir "simd.txt")
