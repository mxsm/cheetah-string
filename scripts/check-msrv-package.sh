#!/usr/bin/env bash
set -euo pipefail

MSRV=${1:-1.75.0}
VERSION=$(cargo metadata --no-deps --format-version 1 |
  python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')
# `cargo package` always stages the unpacked source under `target/package`;
# CARGO_TARGET_DIR applies to compilation artifacts, not this package staging area.
PACKAGE_DIR="target/package/cheetah-string-$VERSION"

cargo package --no-verify
[ -d "$PACKAGE_DIR" ] || {
  echo "packaged source directory is missing: $PACKAGE_DIR" >&2
  exit 1
}
PACKAGE_DIR=$(cd "$PACKAGE_DIR" && pwd)

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

check_consumer() {
  NAME=$1
  FEATURES=$2
  CONSUMER="$TMP/$NAME"
  cargo "+$MSRV" new --lib --name "$NAME" "$CONSUMER" >/dev/null
  if [ -n "$FEATURES" ]; then
    cargo "+$MSRV" add \
      --manifest-path "$CONSUMER/Cargo.toml" \
      --path "$PACKAGE_DIR" \
      --no-default-features \
      --features "$FEATURES" >/dev/null
  else
    cargo "+$MSRV" add \
      --manifest-path "$CONSUMER/Cargo.toml" \
      --path "$PACKAGE_DIR" \
      --no-default-features >/dev/null
  fi
  cargo "+$MSRV" check --manifest-path "$CONSUMER/Cargo.toml"
}

check_consumer cheetah_msrv_no_default ""
check_consumer cheetah_msrv_all_features \
  "std,serde,bytes,experimental-packed,experimental-simd,simd"
check_consumer cheetah_msrv_optional_no_std "serde,bytes"
