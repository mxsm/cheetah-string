$Version = if ($args.Count -gt 0) { $args[0] } else { "current" }
$ResultDir = Join-Path "bench-results" $Version

New-Item -ItemType Directory -Force -Path $ResultDir | Out-Null

cargo test layout_snapshot --all-features -- --nocapture |
    Tee-Object -FilePath (Join-Path $ResultDir "layout-test.txt")

cargo bench --bench layout |
    Tee-Object -FilePath (Join-Path $ResultDir "layout-bench.txt")

cargo bench --bench comprehensive |
    Tee-Object -FilePath (Join-Path $ResultDir "comprehensive.txt")

cargo bench --bench mutation |
    Tee-Object -FilePath (Join-Path $ResultDir "mutation.txt")

cargo bench --bench mq_topic |
    Tee-Object -FilePath (Join-Path $ResultDir "mq-topic.txt")

cargo bench --bench mq_properties |
    Tee-Object -FilePath (Join-Path $ResultDir "mq-properties.txt")

cargo bench --bench mq_remoting_header |
    Tee-Object -FilePath (Join-Path $ResultDir "mq-remoting-header.txt")

cargo bench --bench pattern |
    Tee-Object -FilePath (Join-Path $ResultDir "pattern.txt")

cargo bench --bench simd --features simd |
    Tee-Object -FilePath (Join-Path $ResultDir "simd.txt")
