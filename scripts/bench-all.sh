#!/usr/bin/env sh
set -eu

VERSION="${1:-current}"
RESULT_DIR="bench-results/${VERSION}"

mkdir -p "$RESULT_DIR"

cargo test layout_snapshot --all-features -- --nocapture | tee "$RESULT_DIR/layout-test.txt"
cargo bench --bench layout | tee "$RESULT_DIR/layout-bench.txt"
cargo bench --bench comprehensive | tee "$RESULT_DIR/comprehensive.txt"
cargo bench --bench mutation | tee "$RESULT_DIR/mutation.txt"
cargo bench --bench mq_topic | tee "$RESULT_DIR/mq-topic.txt"
cargo bench --bench mq_properties | tee "$RESULT_DIR/mq-properties.txt"
cargo bench --bench mq_remoting_header | tee "$RESULT_DIR/mq-remoting-header.txt"
cargo bench --bench pattern | tee "$RESULT_DIR/pattern.txt"
cargo bench --bench simd --features simd | tee "$RESULT_DIR/simd.txt"
