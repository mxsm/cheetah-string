#!/usr/bin/env bash
set -euo pipefail

# This capture harness is intentionally separate from bench-all.sh. It runs only
# against the normalized v2 baseline whose runtime source is identical to
# 29315cae, but whose benchmark workloads and allocation/layout contracts were
# repaired before the v3 implementation work started.
EXPECTED_FROZEN_SHA=4a66e95d9ee429a24a55f9a15facad91ac0dec20
CAPTURE_SCHEMA=cheetah-string-capture-v2

VERSION=${1:-current}
SAMPLE_COUNT=${2:-100}
FEATURES=${3:-default}
MODE=${4:-full}
TOOLCHAIN=${CARGO_TOOLCHAIN:-}
RESULT_DIR="bench-results/${VERSION}"
CRITERION_DESTINATION="$RESULT_DIR/criterion"
TARGET_ROOT=${CARGO_TARGET_DIR:-target}
CRITERION_SOURCE="$TARGET_ROOT/criterion"

case "$SAMPLE_COUNT" in
  ''|*[!0-9]*)
    echo "sample count must be a positive integer" >&2
    exit 2
    ;;
esac
[ "$SAMPLE_COUNT" -gt 0 ] || {
  echo "sample count must be a positive integer" >&2
  exit 2
}
case "$MODE" in
  full|smoke) ;;
  *)
    echo "mode must be 'full' or 'smoke'" >&2
    exit 2
    ;;
esac
case "$FEATURES" in
  default) ;;
  *)
    echo "feature profile must be 'default'; the SIMD benchmark is captured separately" >&2
    exit 2
    ;;
esac

[ ! -e "$RESULT_DIR" ] || {
  echo "benchmark capture already exists: $RESULT_DIR" >&2
  exit 1
}
[ ! -e "$CRITERION_SOURCE" ] || {
  echo "Criterion output already exists; use a fresh CARGO_TARGET_DIR: $CRITERION_SOURCE" >&2
  exit 1
}
GIT_SHA=$(git rev-parse HEAD)
[ "$GIT_SHA" = "$EXPECTED_FROZEN_SHA" ] || {
  echo "frozen v2 harness requires $EXPECTED_FROZEN_SHA, found $GIT_SHA" >&2
  exit 1
}
if [ -n "$(git status --porcelain)" ]; then
  GIT_DIRTY=true
else
  GIT_DIRTY=false
fi

git_object_or_absent() {
  PATH_IN_REPOSITORY=$1
  if git cat-file -e "$GIT_SHA:$PATH_IN_REPOSITORY" 2>/dev/null; then
    git rev-parse "$GIT_SHA:$PATH_IN_REPOSITORY"
  else
    printf 'absent\n'
  fi
}

BENCHMARK_TREE=$(git_object_or_absent benches)
RUNTIME_TREE=$(git_object_or_absent src)
ALLOCATION_CONTRACT_BLOB=$(git_object_or_absent tests/allocation_contract.rs)
LAYOUT_CONTRACT_BLOB=$(git_object_or_absent tests/layout_snapshot.rs)
CARGO_TOML_BLOB=$(git_object_or_absent Cargo.toml)
CARGO_CONFIG_TREE=$(git_object_or_absent .cargo)
BUILD_SCRIPT_BLOB=$(git_object_or_absent build.rs)
mkdir -p "$RESULT_DIR"

if [ -n "$TOOLCHAIN" ]; then
  CARGO_PREFIX="+$TOOLCHAIN"
  RUSTC_PREFIX="+$TOOLCHAIN"
else
  CARGO_PREFIX=
  RUSTC_PREFIX=
fi

RUSTC_VERBOSE=$(rustc $RUSTC_PREFIX -vV)
RUSTC_LINE=$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n '1p')
LLVM=$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^LLVM version: //p')
OS=$(uname -srm)
if [ -r /proc/cpuinfo ]; then
  CPU=$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo | sed -n '1p')
else
  CPU=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m)
fi

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

{
  printf '{\n'
  printf '  "schema_version": 1,\n'
  printf '  "capture_schema_version": "%s",\n' "$CAPTURE_SCHEMA"
  printf '  "benchmark_schema_version": "cheetah-string-bench-v1",\n'
  printf '  "criterion_schema_version": "criterion-0.5",\n'
  printf '  "crate": "cheetah-string",\n'
  printf '  "git_sha": "%s",\n' "$(json_escape "$GIT_SHA")"
  printf '  "runtime_tree": "%s",\n' "$RUNTIME_TREE"
  printf '  "git_dirty": %s,\n' "$GIT_DIRTY"
  printf '  "rustc": "%s",\n' "$(json_escape "$RUSTC_LINE")"
  printf '  "llvm": "%s",\n' "$(json_escape "$LLVM")"
  printf '  "cpu": "%s",\n' "$(json_escape "$CPU")"
  printf '  "os": "%s",\n' "$(json_escape "$OS")"
  printf '  "features": "%s",\n' "$(json_escape "$FEATURES;simd-bench=isolated")"
  printf '  "simd_feature_alias": "simd",\n'
  printf '  "harness_identity": {\n'
  printf '    "benchmark_tree": "%s",\n' "$BENCHMARK_TREE"
  printf '    "allocation_contract_blob": "%s",\n' "$ALLOCATION_CONTRACT_BLOB"
  printf '    "layout_contract_blob": "%s",\n' "$LAYOUT_CONTRACT_BLOB"
  printf '    "cargo_toml_blob": "%s",\n' "$CARGO_TOML_BLOB"
  printf '    "cargo_config_tree": "%s",\n' "$CARGO_CONFIG_TREE"
  printf '    "build_script_blob": "%s"\n' "$BUILD_SCRIPT_BLOB"
  printf '  },\n'
  printf '  "sample_count": %s,\n' "$SAMPLE_COUNT"
  if [ "$MODE" = "smoke" ]; then
    printf '  "smoke": true,\n'
  else
    printf '  "smoke": false,\n'
  fi
  printf '  "profile": "bench",\n'
  printf '  "benchmark_ids": ["layout","comprehensive","mutation","mq_topic","mq_properties","mq_remoting_header","pattern","simd","shared_backing"]\n'
  printf '}\n'
} > "$RESULT_DIR/metadata.json"

run_cargo() {
  OUTPUT=$1
  shift
  cargo $CARGO_PREFIX "$@" 2>&1 | tee "$RESULT_DIR/$OUTPUT"
}

require_test_passed() {
  OUTPUT=$1
  grep -Eq 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$RESULT_DIR/$OUTPUT" || {
    echo "contract command did not execute a passing test: $RESULT_DIR/$OUTPUT" >&2
    exit 1
  }
}

if [ "$MODE" = "smoke" ]; then
  CRITERION_ARGS="--sample-size $SAMPLE_COUNT --warm-up-time 0.05 --measurement-time 0.10"
else
  CRITERION_ARGS="--sample-size $SAMPLE_COUNT"
fi

run_cargo layout-test.txt test --test layout_snapshot --all-features -- --nocapture
require_test_passed layout-test.txt
run_cargo allocation-contract.txt test --test allocation_contract --all-features -- --test-threads=1
require_test_passed allocation-contract.txt
cat > "$RESULT_DIR/contracts.json" <<'JSON'
{
  "schema_version": 1,
  "layout_contract": "passed",
  "allocation_contract": "passed",
  "clone_allocations_max": 1,
  "source": "tests/allocation_contract.rs",
  "note": "The normalized v2 baseline preserves the construction-dependent Owned clone."
}
JSON
# CRITERION_ARGS is intentionally word-split into Criterion's individual options.
# shellcheck disable=SC2086
run_cargo layout-bench.txt bench --bench layout -- $CRITERION_ARGS
# shellcheck disable=SC2086
run_cargo comprehensive.txt bench --bench comprehensive -- $CRITERION_ARGS
# shellcheck disable=SC2086
run_cargo mutation.txt bench --bench mutation -- $CRITERION_ARGS
# shellcheck disable=SC2086
run_cargo mq-topic.txt bench --bench mq_topic -- $CRITERION_ARGS
# shellcheck disable=SC2086
run_cargo mq-properties.txt bench --bench mq_properties -- $CRITERION_ARGS
# shellcheck disable=SC2086
run_cargo mq-remoting-header.txt bench --bench mq_remoting_header -- $CRITERION_ARGS
# shellcheck disable=SC2086
run_cargo pattern.txt bench --bench pattern -- $CRITERION_ARGS
# shellcheck disable=SC2086
run_cargo simd.txt bench --bench simd --features simd -- $CRITERION_ARGS
# shellcheck disable=SC2086
run_cargo shared-backing.txt bench --bench shared_backing -- $CRITERION_ARGS

[ -d "$CRITERION_SOURCE" ] || {
  echo "Criterion result directory is missing: $CRITERION_SOURCE" >&2
  exit 1
}
cp -R "$CRITERION_SOURCE" "$CRITERION_DESTINATION"
