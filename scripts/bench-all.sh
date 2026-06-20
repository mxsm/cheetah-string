#!/usr/bin/env sh
set -eu

VERSION="${1:-current}"
RESULT_DIR="bench-results/${VERSION}"

mkdir -p "$RESULT_DIR"

cargo test layout_snapshot --all-features -- --nocapture | tee "$RESULT_DIR/layout-test.txt"
cargo bench --bench layout | tee "$RESULT_DIR/layout-bench.txt"
cargo bench --bench comprehensive | tee "$RESULT_DIR/comprehensive.txt"
cargo bench --bench simd --features simd | tee "$RESULT_DIR/simd.txt"
