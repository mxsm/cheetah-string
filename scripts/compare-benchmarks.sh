#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <base-result-dir> <head-result-dir> [manifest.json] [output-dir] [pr|final]" >&2
  exit 2
fi

BASE=$1
HEAD=$2
MANIFEST=${3:-bench-results/gates/v3-score-gates.json}
OUTPUT=${4:-bench-results/comparisons/local}
MODE=${5:-pr}
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

case "$MODE" in
  pr|final) ;;
  *)
    echo "mode must be 'pr' or 'final'" >&2
    exit 2
    ;;
esac

command -v jq >/dev/null 2>&1 || {
  echo "jq is required to compare benchmark evidence" >&2
  exit 2
}
command -v python3 >/dev/null 2>&1 || {
  echo "python3 is required to verify the score ledger" >&2
  exit 2
}

[ -f "$MANIFEST" ] || {
  echo "gate manifest is missing: $MANIFEST" >&2
  exit 1
}

validate_metadata() {
  DIR=$1
  LABEL=$2
  FILE="$DIR/metadata.json"
  [ -f "$FILE" ] || {
    echo "$LABEL metadata is missing: $FILE" >&2
    exit 1
  }
  jq -e '
    (.schema_version == 1) and
    (.crate == "cheetah-string") and
    (.git_dirty == false) and
    (.smoke == false) and
    ([.git_sha,.runtime_tree,.rustc,.llvm,.cpu,.os,.features,.simd_feature_alias,.capture_schema_version,.benchmark_schema_version,.criterion_schema_version] |
      all(type == "string" and length > 0)) and
    (.harness_identity | type == "object") and
    ([.harness_identity.benchmark_tree,
      .harness_identity.allocation_contract_blob,
      .harness_identity.layout_contract_blob,
      .harness_identity.cargo_toml_blob,
      .harness_identity.cargo_config_tree,
      .harness_identity.build_script_blob] |
      all(type == "string" and length > 0)) and
    (.sample_count | type == "number" and . > 0)
  ' "$FILE" >/dev/null || {
    echo "$LABEL metadata is incomplete or inadmissible: $FILE" >&2
    exit 1
  }
}

validate_metadata "$BASE" base
validate_metadata "$HEAD" head

validate_harness_identity() {
  DIR=$1
  LABEL=$2
  MANIFEST_QUERY=$3
  EXPECTED=$(jq -cS "$MANIFEST_QUERY" "$MANIFEST")
  ACTUAL=$(jq -cS '.harness_identity' "$DIR/metadata.json")
  [ "$ACTUAL" = "$EXPECTED" ] || {
    echo "$LABEL harness identity is not the protected policy version" >&2
    exit 1
  }
}

validate_harness_identity "$HEAD" head '.protected_candidate_harness'
BASE_SHA=$(jq -er '.git_sha' "$BASE/metadata.json")
FROZEN_SHA=$(jq -er '.frozen_baseline.git_sha' "$MANIFEST")
if [ "$BASE_SHA" = "$FROZEN_SHA" ]; then
  validate_harness_identity "$BASE" base '.frozen_baseline.harness_identity'
  EXPECTED_RUNTIME_TREE=$(jq -er '.frozen_baseline.runtime_source_tree' "$MANIFEST")
  ACTUAL_RUNTIME_TREE=$(jq -er '.runtime_tree' "$BASE/metadata.json")
  [ "$ACTUAL_RUNTIME_TREE" = "$EXPECTED_RUNTIME_TREE" ] || {
    echo "frozen base runtime tree does not match the versioned source identity" >&2
    exit 1
  }
else
  validate_harness_identity "$BASE" base '.protected_candidate_harness'
fi

for FIELD in rustc llvm cpu os features sample_count capture_schema_version benchmark_schema_version criterion_schema_version; do
  BASE_VALUE=$(jq -r ".$FIELD" "$BASE/metadata.json")
  HEAD_VALUE=$(jq -r ".$FIELD" "$HEAD/metadata.json")
  [ "$BASE_VALUE" = "$HEAD_VALUE" ] || {
    echo "base/head metadata mismatch for '$FIELD'" >&2
    exit 1
  }
done

MINIMUM_SAMPLE_COUNT=$(jq -er '.frozen_baseline.minimum_sample_count' "$MANIFEST")
BASE_SAMPLE_COUNT=$(jq -er '.sample_count' "$BASE/metadata.json")
[ "$BASE_SAMPLE_COUNT" -ge "$MINIMUM_SAMPLE_COUNT" ] || {
  echo "base sample_count is below the blocking minimum of $MINIMUM_SAMPLE_COUNT" >&2
  exit 1
}
EXPECTED_CRITERION_SCHEMA=$(jq -er '.frozen_baseline.criterion_schema_version' "$MANIFEST")
ACTUAL_CRITERION_SCHEMA=$(jq -er '.criterion_schema_version' "$BASE/metadata.json")
[ "$ACTUAL_CRITERION_SCHEMA" = "$EXPECTED_CRITERION_SCHEMA" ] || {
  echo "Criterion schema does not match the gate manifest" >&2
  exit 1
}
EXPECTED_CAPTURE_SCHEMA=$(jq -er '.frozen_baseline.capture_schema_version' "$MANIFEST")
ACTUAL_CAPTURE_SCHEMA=$(jq -er '.capture_schema_version' "$BASE/metadata.json")
[ "$ACTUAL_CAPTURE_SCHEMA" = "$EXPECTED_CAPTURE_SCHEMA" ] || {
  echo "capture schema does not match the gate manifest" >&2
  exit 1
}
if [ "$MODE" = "final" ]; then
  [ "$BASE_SHA" = "$FROZEN_SHA" ] || {
    echo "final comparison base must be the frozen baseline $FROZEN_SHA" >&2
    exit 1
  }
fi

[ -f "$HEAD/contracts.json" ] || {
  echo "head contract evidence is missing: $HEAD/contracts.json" >&2
  exit 1
}
jq -e '
  .layout_contract == "passed" and
  .allocation_contract == "passed" and
  (.clone_allocations_max | type == "number")
' "$HEAD/contracts.json" >/dev/null || {
  echo "head deterministic contracts did not pass" >&2
  exit 1
}

mkdir -p "$OUTPUT"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

collect_medians() {
  DIR=$1
  LABEL=$2
  DEST=$3
  CRITERION="$DIR/criterion"
  [ -d "$CRITERION" ] || {
    echo "$LABEL Criterion capture is missing: $CRITERION" >&2
    exit 1
  }

  : > "$DEST.jsonl"
  find "$CRITERION" -type f -path '*/new/benchmark.json' | sort |
    while IFS= read -r BENCHMARK; do
      BENCHMARK_DIR=$(dirname "$BENCHMARK")
      ESTIMATE="$BENCHMARK_DIR/estimates.json"
      [ -f "$ESTIMATE" ] || {
        echo "$LABEL estimate is missing beside $BENCHMARK" >&2
        exit 1
      }
      ID=$(jq -er '.full_id | select(type == "string" and length > 0)' "$BENCHMARK")
      MEDIAN=$(jq -er '.median.point_estimate | select(type == "number" and . > 0)' "$ESTIMATE")
      jq -cn --arg id "$ID" --argjson median "$MEDIAN" '{key:$id,value:$median}' >> "$DEST.jsonl"
    done

  [ -s "$DEST.jsonl" ] || {
    echo "$LABEL Criterion capture contains no benchmark records" >&2
    exit 1
  }
  jq -s '
    if (map(.key) | length) != (map(.key) | unique | length)
    then error("duplicate benchmark id")
    else from_entries
    end
  ' "$DEST.jsonl" > "$DEST"
}

collect_medians "$BASE" base "$TMP/base.json"
collect_medians "$HEAD" head "$TMP/head.json"

jq -n \
  --arg mode "$MODE" \
  --slurpfile gate "$MANIFEST" \
  --slurpfile base_metadata "$BASE/metadata.json" \
  --slurpfile head_metadata "$HEAD/metadata.json" \
  --slurpfile base "$TMP/base.json" \
  --slurpfile head "$TMP/head.json" \
  --slurpfile contracts "$HEAD/contracts.json" '
  def ids_for($workload; $values):
    if ($workload | has("id")) then
      [$workload.id]
    else
      [$values | keys[] | select(startswith($workload.id_prefix))]
    end;

  $gate[0] as $gate |
  $base[0] as $base |
  $head[0] as $head |
  $contracts[0] as $contracts |
  (if $mode == "final"
   then $gate.comparison_policy.final_vs_frozen_baseline
   else $gate.comparison_policy.pr
   end) as $policy |

  ([
    $gate.core_workloads[] as $workload |
    (ids_for($workload; $head)) as $ids |
    if ($ids | length) == 0 then
      {
        id: $workload.id_prefix,
        weight: $workload.weight,
        base_median_ns: null,
        head_median_ns: null,
        ratio: null,
        passed: false,
        reason: "missing benchmark id"
      }
    else
      $ids[] as $id |
      ($base[$id]) as $base_value |
      ($head[$id]) as $head_value |
      (if ($base_value != null and $base_value > 0 and $head_value != null)
       then ($head_value / $base_value)
       else null
       end) as $ratio |
      {
        id: $id,
        weight: ($workload.weight / ($ids | length)),
        base_median_ns: $base_value,
        head_median_ns: $head_value,
        ratio: $ratio,
        passed: ($ratio != null and $ratio <= $policy.critical_workload_max_ratio),
        reason: (if $ratio == null then "benchmark missing from base or head" else null end)
      }
    end
  ]) as $core |
  ($core | map(select(.ratio != null))) as $valid_core |
  (if ($valid_core | length) > 0 then
    (($valid_core | map(.weight * (.ratio | log)) | add) /
      ($valid_core | map(.weight) | add) | exp)
   else null
   end) as $geomean |
  (($geomean != null) and
   ($geomean <= $policy.core_weighted_geomean_max_ratio) and
   (all($core[]; .passed))) as $core_passed |

  ([
    $gate.absolute_budgets[] as $budget |
    ($head[$budget.id]) as $head_value |
    (if ($budget | has("reference_id")) then $head[$budget.reference_id] else null end) as $reference_value |
    (if ($head_value != null and $reference_value != null and $reference_value > 0)
     then ($head_value / $reference_value)
     else null
     end) as $ratio |
    ([
      if ($budget | has("max_ratio"))
      then ($ratio != null and $ratio <= $budget.max_ratio)
      else empty end,
      if ($budget | has("max_median_ns"))
      then ($head_value != null and $head_value <= $budget.max_median_ns)
      else empty end,
      if ($budget | has("max_allocations"))
      then ($contracts.clone_allocations_max <= $budget.max_allocations)
      else empty end
    ]) as $checks |
    {
      id: $budget.id,
      reference_id: ($budget.reference_id // null),
      head_median_ns: $head_value,
      reference_median_ns: $reference_value,
      ratio: $ratio,
      max_ratio: ($budget.max_ratio // null),
      max_median_ns: ($budget.max_median_ns // null),
      clone_allocations:
        (if ($budget | has("max_allocations")) then $contracts.clone_allocations_max else null end),
      passed: (($checks | length) > 0 and all($checks[]; .))
    }
  ]) as $absolute |
  (all($absolute[]; .passed)) as $absolute_passed |

  ([
    $gate.score.conditions[] |
    . as $condition |
    (if $condition.id == "performance-zero-allocation-clone" then
      ($contracts.clone_allocations_max == 0) as $passed |
      {status:(if $passed then "passed" else "failed" end), passed:$passed}
    elif $condition.id == "performance-final-geomean" then
      if $mode == "final"
      then {status:(if $core_passed then "passed" else "failed" end), passed:$core_passed}
      else {status:"final-comparison-required", passed:false}
      end
    elif $condition.id == "performance-fixed-runner-budgets" then
      (($core_passed and $absolute_passed)) as $passed |
      {status:(if $passed then "passed" else "failed" end), passed:$passed}
    else
      {status:"external-evidence-required", passed:false}
    end) as $assessment |
    {
      id: $condition.id,
      dimension: $condition.dimension,
      points: $condition.points,
      status: $assessment.status,
      awarded: (if $assessment.passed then $condition.points else 0 end)
    }
  ]) as $ledger |

  {
    schema_version: 2,
    manifest_id: $gate.manifest_id,
    mode: $mode,
    base_git_sha: $base_metadata[0].git_sha,
    head_git_sha: $head_metadata[0].git_sha,
    metadata_compatible: true,
    policy: {
      core_weighted_geomean_max_ratio: $policy.core_weighted_geomean_max_ratio,
      critical_workload_max_ratio: $policy.critical_workload_max_ratio
    },
    core_weighted_geomean_ratio: $geomean,
    core_workloads: $core,
    absolute_budgets: $absolute,
    score_ledger: $ledger,
    automatic_comparison_score:
      ($gate.score.baseline + ($ledger | map(.awarded) | add)),
    performance_verdict:
      (if ($core_passed and $absolute_passed) then "pass" else "fail" end)
  }
' > "$OUTPUT/comparison.raw.json"

VERIFY_ARGS=(
  --manifest "$MANIFEST"
  --comparison "$OUTPUT/comparison.raw.json"
  --mode "$MODE"
  --output "$OUTPUT/comparison.json"
)
if python3 "$SCRIPT_DIR/verify-score.py" "${VERIFY_ARGS[@]}" > "$OUTPUT/score-verifier.txt"; then
  :
else
  SCORE_EXIT=$?
  if ! jq -e '.manifest_id' "$OUTPUT/comparison.json" >/dev/null 2>&1; then
    {
      echo "# Benchmark comparison"
      echo
      echo "- Verdict: **FAIL**"
      echo "- Score verifier rejected the comparison evidence."
      echo
      jq -r '.errors[]? | "- " + .' "$OUTPUT/comparison.json"
    } > "$OUTPUT/comparison.md"
    cat "$OUTPUT/comparison.json"
    exit "$SCORE_EXIT"
  fi
fi

{
  echo "# Benchmark comparison"
  echo
  jq -r '
    "- Verdict: **\(.verdict | ascii_upcase)**\n" +
    "- Mode: `\(.mode)`\n" +
    "- Base: `\(.base_git_sha)`\n" +
    "- Head: `\(.head_git_sha)`\n" +
    "- Gate scope: `\(.gate_scope)`\n" +
    "- Score: `\(.score.total)/\(.score.target)` (`\(.score_verdict)`)\n" +
    "- Weighted geometric mean: `\(.core_weighted_geomean_ratio)` " +
    "(limit `\(.policy.core_weighted_geomean_max_ratio)`)"
  ' "$OUTPUT/comparison.json"
  echo
  echo "## Core workloads"
  echo
  echo "| Benchmark | Base ns | Head ns | Ratio | Result |"
  echo "|---|---:|---:|---:|---|"
  jq -r '.core_workloads[] |
    "| \(.id | gsub("\\|"; "\\|")) | \(.base_median_ns) | \(.head_median_ns) | \(.ratio) | \(if .passed then "PASS" else "FAIL" end) |"
  ' "$OUTPUT/comparison.json"
  echo
  echo "## Absolute budgets"
  echo
  echo "| Benchmark | Median ns | Ratio | Max ratio | Max ns | Result |"
  echo "|---|---:|---:|---:|---:|---|"
  jq -r '.absolute_budgets[] |
    "| \(.id | gsub("\\|"; "\\|")) | \(.head_median_ns) | \(.ratio) | \(.max_ratio) | \(.max_median_ns) | \(if .passed then "PASS" else "FAIL" end) |"
  ' "$OUTPUT/comparison.json"
  echo
  echo "## Score ledger"
  echo
  echo "| Condition | Dimension | Points | Status | Awarded |"
  echo "|---|---|---:|---|---:|"
  jq -r '.score_ledger[] |
    "| \(.id) | \(.dimension) | \(.points) | \(.status) | \(.awarded) |"
  ' "$OUTPUT/comparison.json"
} > "$OUTPUT/comparison.md"

cat "$OUTPUT/comparison.json"
[ "$(jq -r .verdict "$OUTPUT/comparison.json")" = "pass" ]
