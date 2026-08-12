#!/usr/bin/env bash
set -euo pipefail

MSRV=${1:-1.95}
VERSION=$(cargo metadata --no-deps --format-version 1 |
  python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')
CRATE_ARCHIVE="target/package/cheetah-string-$VERSION.crate"

cargo package --no-verify
[ -f "$CRATE_ARCHIVE" ] || {
  echo "package archive is missing: $CRATE_ARCHIVE" >&2
  exit 1
}
CRATE_ARCHIVE=$(cd "$(dirname "$CRATE_ARCHIVE")" && pwd)/$(basename "$CRATE_ARCHIVE")

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
tar -xzf "$CRATE_ARCHIVE" -C "$TMP"
PACKAGE_DIR="$TMP/cheetah-string-$VERSION"
[ -d "$PACKAGE_DIR" ] || {
  echo "unpacked package directory is missing: $PACKAGE_DIR" >&2
  exit 1
}

check_consumer() {
  NAME=$1
  FEATURES=$2
  CONSUMER="$TMP/consumer-$NAME"
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
