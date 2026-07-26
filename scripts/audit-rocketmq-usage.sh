#!/usr/bin/env sh
set -eu

if [ "$#" -lt 1 ]; then
  echo "usage: $0 <read-only-downstream-checkout> [output.json]" >&2
  exit 2
fi

RG=${RG:-rg}
for command_name in git "$RG" cargo jq; do
  command -v "$command_name" >/dev/null 2>&1 || {
    echo "$command_name is required" >&2
    exit 2
  }
done

REPO=$1
OUT=${2:-bench-results/usage-inventory/rocketmq-current.json}
GLOB='**/src/**/*.rs'
[ -d "$REPO" ] || {
  echo "downstream checkout does not exist: $REPO" >&2
  exit 2
}
SHA=$(git -C "$REPO" rev-parse HEAD)
GENERATED_AT=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
COMMANDS_JSONL="$TMP/commands.jsonl"
METADATA_JSON="$TMP/metadata.json"
: > "$COMMANDS_JSONL"

measure_pattern() {
  NAME=$1
  PATTERN=$2
  OCCURRENCES=$("$RG" -o --glob "$GLOB" "$PATTERN" "$REPO" 2>/dev/null | wc -l | tr -d ' ')
  FILES=$("$RG" -l --glob "$GLOB" "$PATTERN" "$REPO" 2>/dev/null | wc -l | tr -d ' ')
  jq -cn \
    --arg name "$NAME" \
    --arg pattern "$PATTERN" \
    --arg command "rg -o --glob '$GLOB' '$PATTERN' <repo>" \
    --argjson occurrences "$OCCURRENCES" \
    --argjson files "$FILES" \
    '{name:$name,pattern:$pattern,command:$command,occurrences:$occurrences,files:$files}' \
    >> "$COMMANDS_JSONL"
}

measure_pattern CheetahString 'CheetahString'
measure_pattern CheetahStr 'CheetahStr\b'
measure_pattern CheetahBuilder 'CheetahBuilder'
measure_pattern constructors 'CheetahString::(from|from_slice|from_string|from_static_str|try_from_vec|try_from_bytes)\b'
measure_pattern finish_string 'finish_string\s*\('
measure_pattern from_string_owned 'from_string_owned\s*\('
measure_pattern with_capacity 'CheetahString::with_capacity\s*\('
measure_pattern push_str_lexical '\.push_str\s*\('
measure_pattern reserve_lexical '\.reserve\s*\('
measure_pattern typed_split '\.(split_char|split_str)\s*\('
measure_pattern collection_key '(HashMap|HashSet)\s*<[^>]*CheetahString'
measure_pattern trait_bound '(Into|From|AsRef|Borrow)\s*<\s*CheetahString\s*>'
measure_pattern public_signature 'pub(\([^)]*\))?\s+(async\s+)?fn[^{;]*CheetahString'
measure_pattern CheetahBytes 'CheetahBytes'

cargo metadata --locked --manifest-path "$REPO/Cargo.toml" --no-deps --format-version 1 \
  > "$METADATA_JSON"

mkdir -p "$(dirname "$OUT")"
jq -n \
  --arg repository rocketmq-rust \
  --arg git_sha "$SHA" \
  --arg generated_at_utc "$GENERATED_AT" \
  --arg glob "$GLOB" \
  --slurpfile commands "$COMMANDS_JSONL" \
  --slurpfile metadata "$METADATA_JSON" '
  (
    $metadata[0].packages
    | map(select(any(.dependencies[]; .name == "cheetah-string")) | .name)
    | sort
  ) as $workspace_names |
  {
    schema_version: 2,
    repository: $repository,
    git_sha: $git_sha,
    generated_at_utc: $generated_at_utc,
    scope: {
      glob: $glob,
      kind: "lexical inventory",
      gitignore_rules_honored: true,
      explicit_exclusions: [],
      includes_comments: true,
      includes_imports: true,
      includes_cfg_test: true,
      not_a_runtime_profile: true,
      not_type_resolved: true
    },
    commands: $commands,
    dependency_packages: {
      workspace: ($workspace_names | length),
      standalone: 2,
      total: (($workspace_names | length) + 2),
      workspace_names: $workspace_names,
      standalone_names: ["rocketmq-example", "rocketmq-dashboard-backend"]
    }
  }' > "$OUT"

echo "Wrote $OUT for $SHA"
