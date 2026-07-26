param(
    [Parameter(Mandatory = $true)]
    [string]$Base,
    [Parameter(Mandatory = $true)]
    [string]$Head,
    [string]$Manifest = "bench-results/gates/v3-score-gates.json",
    [string]$OutputDirectory = "bench-results/comparisons/local",
    [ValidateSet("pr", "final")]
    [string]$Mode = "pr"
)

$ErrorActionPreference = "Stop"
$requiredMetadata = @(
    "schema_version",
    "git_sha",
    "runtime_tree",
    "git_dirty",
    "rustc",
    "llvm",
    "cpu",
    "os",
    "features",
    "simd_feature_alias",
    "sample_count",
    "smoke",
    "capture_schema_version",
    "benchmark_schema_version",
    "criterion_schema_version"
)
$requiredHarnessFields = @(
    "benchmark_tree",
    "allocation_contract_blob",
    "layout_contract_blob",
    "cargo_toml_blob",
    "cargo_config_tree",
    "build_script_blob"
)

function Read-CompleteMetadata {
    param([string]$Directory, [string]$Label)

    $path = Join-Path $Directory "metadata.json"
    if (-not (Test-Path -LiteralPath $path)) {
        throw "$Label metadata is missing: $path"
    }
    $metadata = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    foreach ($field in $requiredMetadata) {
        $value = $metadata.$field
        if ($null -eq $value -or ($value -is [string] -and [string]::IsNullOrWhiteSpace($value))) {
            throw "$Label metadata field '$field' is missing or empty"
        }
    }
    if ([int]$metadata.sample_count -le 0) {
        throw "$Label sample_count must be positive"
    }
    if ([bool]$metadata.git_dirty) {
        throw "$Label capture was produced from a dirty worktree"
    }
    if ($metadata.smoke -eq $true) {
        throw "$Label smoke capture cannot pass a performance comparison"
    }
    if ([int]$metadata.schema_version -ne 1 -or [string]$metadata.crate -ne "cheetah-string") {
        throw "$Label metadata has an unsupported schema or crate identity"
    }
    if ($null -eq $metadata.harness_identity) {
        throw "$Label metadata has no harness identity"
    }
    foreach ($field in $requiredHarnessFields) {
        $value = [string]$metadata.harness_identity.$field
        if ([string]::IsNullOrWhiteSpace($value)) {
            throw "$Label harness identity field '$field' is missing or empty"
        }
    }
    return $metadata
}

function Assert-HarnessIdentity {
    param(
        [object]$Metadata,
        [object]$Expected,
        [string]$Label
    )

    foreach ($field in $requiredHarnessFields) {
        if ([string]$Metadata.harness_identity.$field -ne [string]$Expected.$field) {
            throw "$Label harness identity is not the protected policy version: $field"
        }
    }
}

function Read-CriterionMedians {
    param([string]$Directory, [string]$Label)

    $criterion = Join-Path $Directory "criterion"
    if (-not (Test-Path -LiteralPath $criterion)) {
        throw "$Label Criterion capture is missing: $criterion"
    }

    $medians = @{}
    $benchmarks = Get-ChildItem -LiteralPath $criterion -Recurse -Filter "benchmark.json" |
        Where-Object { $_.Directory.Name -eq "new" }
    foreach ($benchmarkPath in $benchmarks) {
        $estimatePath = Join-Path $benchmarkPath.Directory.FullName "estimates.json"
        if (-not (Test-Path -LiteralPath $estimatePath)) {
            throw "$Label estimate is missing beside $($benchmarkPath.FullName)"
        }
        $benchmark = Get-Content -Raw -LiteralPath $benchmarkPath.FullName | ConvertFrom-Json
        $estimate = Get-Content -Raw -LiteralPath $estimatePath | ConvertFrom-Json
        $id = [string]$benchmark.full_id
        $median = [double]$estimate.median.point_estimate
        if ([string]::IsNullOrWhiteSpace($id) -or $median -le 0) {
            throw "$Label contains an invalid benchmark record: $($benchmarkPath.FullName)"
        }
        if ($medians.ContainsKey($id)) {
            throw "$Label contains duplicate benchmark id '$id'"
        }
        $medians[$id] = $median
    }
    if ($medians.Count -eq 0) {
        throw "$Label Criterion capture contains no benchmark records"
    }
    return $medians
}

function Read-Contracts {
    param([string]$Directory, [string]$Label)

    $path = Join-Path $Directory "contracts.json"
    if (-not (Test-Path -LiteralPath $path)) {
        throw "$Label contract evidence is missing: $path"
    }
    $contracts = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    if ($contracts.layout_contract -ne "passed" -or $contracts.allocation_contract -ne "passed") {
        throw "$Label deterministic contracts did not pass"
    }
    return $contracts
}

if (-not (Test-Path -LiteralPath $Manifest)) {
    throw "Gate manifest is missing: $Manifest"
}

$gate = Get-Content -Raw -LiteralPath $Manifest | ConvertFrom-Json
$baseMetadata = Read-CompleteMetadata $Base "base"
$headMetadata = Read-CompleteMetadata $Head "head"
Assert-HarnessIdentity $headMetadata $gate.protected_candidate_harness "head"
if ([string]$baseMetadata.git_sha -eq [string]$gate.frozen_baseline.git_sha) {
    Assert-HarnessIdentity $baseMetadata $gate.frozen_baseline.harness_identity "base"
    if ([string]$baseMetadata.runtime_tree -ne [string]$gate.frozen_baseline.runtime_source_tree) {
        throw "frozen base runtime tree does not match the versioned source identity"
    }
} else {
    Assert-HarnessIdentity $baseMetadata $gate.protected_candidate_harness "base"
}
$baseBenchmarks = Read-CriterionMedians $Base "base"
$headBenchmarks = Read-CriterionMedians $Head "head"
$headContracts = Read-Contracts $Head "head"

foreach ($field in @("rustc", "llvm", "cpu", "os", "features", "sample_count", "capture_schema_version", "benchmark_schema_version", "criterion_schema_version")) {
    if ($baseMetadata.$field -ne $headMetadata.$field) {
        throw "base/head metadata mismatch for '$field'"
    }
}

$minimumSampleCount = [int]$gate.frozen_baseline.minimum_sample_count
if ([int]$baseMetadata.sample_count -lt $minimumSampleCount) {
    throw "base sample_count is below the blocking minimum of $minimumSampleCount"
}
if ([string]$baseMetadata.criterion_schema_version -ne [string]$gate.frozen_baseline.criterion_schema_version) {
    throw "Criterion schema does not match the gate manifest"
}
if ([string]$baseMetadata.capture_schema_version -ne [string]$gate.frozen_baseline.capture_schema_version) {
    throw "capture schema does not match the gate manifest"
}
if ($Mode -eq "final" -and
    [string]$baseMetadata.git_sha -ne [string]$gate.frozen_baseline.git_sha) {
    throw "final comparison base must be the frozen baseline $($gate.frozen_baseline.git_sha)"
}

$policy = if ($Mode -eq "final") {
    $gate.comparison_policy.final_vs_frozen_baseline
} else {
    $gate.comparison_policy.pr
}
$geomeanLimit = [double]$policy.core_weighted_geomean_max_ratio
$criticalLimit = [double]$policy.critical_workload_max_ratio

$core = [System.Collections.Generic.List[object]]::new()
foreach ($workload in $gate.core_workloads) {
    $ids = if ($null -ne $workload.id) {
        @([string]$workload.id)
    } else {
        @($headBenchmarks.Keys | Where-Object { $_.StartsWith([string]$workload.id_prefix) } | Sort-Object)
    }

    if ($ids.Count -eq 0) {
        $core.Add([pscustomobject][ordered]@{
            id = [string]$workload.id_prefix
            weight = [double]$workload.weight
            base_median_ns = $null
            head_median_ns = $null
            ratio = $null
            passed = $false
            reason = "missing benchmark id"
        })
        continue
    }

    $itemWeight = [double]$workload.weight / $ids.Count
    foreach ($id in $ids) {
        $baseValue = $baseBenchmarks[$id]
        $headValue = $headBenchmarks[$id]
        $valid = $null -ne $baseValue -and $null -ne $headValue -and [double]$baseValue -gt 0
        $ratio = if ($valid) { [double]$headValue / [double]$baseValue } else { $null }
        $core.Add([pscustomobject][ordered]@{
            id = $id
            weight = $itemWeight
            base_median_ns = if ($null -ne $baseValue) { [double]$baseValue } else { $null }
            head_median_ns = if ($null -ne $headValue) { [double]$headValue } else { $null }
            ratio = $ratio
            passed = $valid -and $ratio -le $criticalLimit
            reason = if ($valid) { $null } else { "benchmark missing from base or head" }
        })
    }
}

$validCore = @($core | Where-Object { $null -ne $_.ratio })
$weightSum = ($validCore | Measure-Object -Property weight -Sum).Sum
$weightedLog = 0.0
foreach ($entry in $validCore) {
    $weightedLog += [double]$entry.weight * [Math]::Log([double]$entry.ratio)
}
$geomean = if ($validCore.Count -gt 0 -and $weightSum -gt 0) {
    [Math]::Exp($weightedLog / $weightSum)
} else {
    $null
}
$corePassed = $null -ne $geomean -and $geomean -le $geomeanLimit -and
    @($core | Where-Object { -not $_.passed }).Count -eq 0

$absolute = [System.Collections.Generic.List[object]]::new()
foreach ($budget in $gate.absolute_budgets) {
    $id = [string]$budget.id
    $headValue = $headBenchmarks[$id]
    $referenceValue = if ($null -ne $budget.reference_id) {
        $headBenchmarks[[string]$budget.reference_id]
    } else {
        $null
    }
    $ratio = if ($null -ne $headValue -and $null -ne $referenceValue -and [double]$referenceValue -gt 0) {
        [double]$headValue / [double]$referenceValue
    } else {
        $null
    }

    $checks = [System.Collections.Generic.List[bool]]::new()
    if ($null -ne $budget.max_ratio) {
        $checks.Add($null -ne $ratio -and $ratio -le [double]$budget.max_ratio)
    }
    if ($null -ne $budget.max_median_ns) {
        $checks.Add($null -ne $headValue -and [double]$headValue -le [double]$budget.max_median_ns)
    }
    if ($null -ne $budget.max_allocations) {
        $checks.Add(
            $null -ne $headContracts.clone_allocations_max -and
            [int]$headContracts.clone_allocations_max -le [int]$budget.max_allocations
        )
    }

    $passed = $checks.Count -gt 0 -and @($checks | Where-Object { -not $_ }).Count -eq 0
    $absolute.Add([pscustomobject][ordered]@{
        id = $id
        reference_id = if ($null -ne $budget.reference_id) { [string]$budget.reference_id } else { $null }
        head_median_ns = if ($null -ne $headValue) { [double]$headValue } else { $null }
        reference_median_ns = if ($null -ne $referenceValue) { [double]$referenceValue } else { $null }
        ratio = $ratio
        max_ratio = if ($null -ne $budget.max_ratio) { [double]$budget.max_ratio } else { $null }
        max_median_ns = if ($null -ne $budget.max_median_ns) { [double]$budget.max_median_ns } else { $null }
        clone_allocations = if ($null -ne $budget.max_allocations) {
            [int]$headContracts.clone_allocations_max
        } else {
            $null
        }
        passed = $passed
    })
}
$absolutePassed = @($absolute | Where-Object { -not $_.passed }).Count -eq 0

$ledger = [System.Collections.Generic.List[object]]::new()
foreach ($condition in $gate.score.conditions) {
    $status = "external-evidence-required"
    $passed = $false
    switch ([string]$condition.id) {
        "performance-zero-allocation-clone" {
            $passed = [int]$headContracts.clone_allocations_max -eq 0
            $status = if ($passed) { "passed" } else { "failed" }
        }
        "performance-final-geomean" {
            if ($Mode -eq "final") {
                $passed = $corePassed
                $status = if ($passed) { "passed" } else { "failed" }
            } else {
                $status = "final-comparison-required"
            }
        }
        "performance-fixed-runner-budgets" {
            $passed = $corePassed -and $absolutePassed
            $status = if ($passed) { "passed" } else { "failed" }
        }
    }
    $ledger.Add([pscustomobject][ordered]@{
        id = [string]$condition.id
        dimension = [string]$condition.dimension
        points = [int]$condition.points
        status = $status
        awarded = if ($passed) { [int]$condition.points } else { 0 }
    })
}

$awardedPoints = ($ledger | Measure-Object -Property awarded -Sum).Sum
$performanceVerdict = if ($corePassed -and $absolutePassed) { "pass" } else { "fail" }
$result = [ordered]@{
    schema_version = 2
    manifest_id = [string]$gate.manifest_id
    mode = $Mode
    base_git_sha = [string]$baseMetadata.git_sha
    head_git_sha = [string]$headMetadata.git_sha
    metadata_compatible = $true
    policy = [ordered]@{
        core_weighted_geomean_max_ratio = $geomeanLimit
        critical_workload_max_ratio = $criticalLimit
    }
    core_weighted_geomean_ratio = $geomean
    core_workloads = $core
    absolute_budgets = $absolute
    score_ledger = $ledger
    automatic_comparison_score = [int]$gate.score.baseline + [int]$awardedPoints
    performance_verdict = $performanceVerdict
}

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$rawJsonPath = Join-Path $OutputDirectory "comparison.raw.json"
$jsonPath = Join-Path $OutputDirectory "comparison.json"
$markdownPath = Join-Path $OutputDirectory "comparison.md"
$json = $result | ConvertTo-Json -Depth 12
$json | Set-Content -Encoding utf8 -LiteralPath $rawJsonPath

$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) {
    $python = Get-Command python3 -ErrorAction SilentlyContinue
}
if (-not $python) {
    throw "python3 or python is required to verify the score ledger"
}
$verifyArguments = @(
    (Join-Path $PSScriptRoot "verify-score.py"),
    "--manifest", $Manifest,
    "--comparison", $rawJsonPath,
    "--mode", $Mode,
    "--output", $jsonPath
)
& $python.Source @verifyArguments 2>&1 |
    Set-Content -Encoding utf8 -LiteralPath (Join-Path $OutputDirectory "score-verifier.txt")
$verifyExitCode = $LASTEXITCODE
$verified = Get-Content -Raw -LiteralPath $jsonPath | ConvertFrom-Json
if (-not $verified.manifest_id) {
    @(
        "# Benchmark comparison",
        "",
        "- Verdict: **FAIL**",
        "- Score verifier rejected the comparison evidence.",
        "",
        @($verified.errors | ForEach-Object { "- $_" })
    ) | Set-Content -Encoding utf8 -LiteralPath $markdownPath
    Write-Output (Get-Content -Raw -LiteralPath $jsonPath)
    exit $(if ($verifyExitCode -ne 0) { $verifyExitCode } else { 1 })
}

$markdown = [System.Text.StringBuilder]::new()
[void]$markdown.AppendLine("# Benchmark comparison")
[void]$markdown.AppendLine()
[void]$markdown.AppendLine("- Verdict: **$(([string]$verified.verdict).ToUpperInvariant())**")
[void]$markdown.AppendLine("- Mode: ``$Mode``")
[void]$markdown.AppendLine("- Base: ``$($baseMetadata.git_sha)``")
[void]$markdown.AppendLine("- Head: ``$($headMetadata.git_sha)``")
[void]$markdown.AppendLine("- Gate scope: ``$($verified.gate_scope)``")
[void]$markdown.AppendLine("- Score: ``$($verified.score.total)/$($verified.score.target)`` (``$($verified.score_verdict)``)")
[void]$markdown.AppendLine("- Weighted geometric mean: ``$geomean`` (limit ``$geomeanLimit``)")
[void]$markdown.AppendLine()
[void]$markdown.AppendLine("## Core workloads")
[void]$markdown.AppendLine()
[void]$markdown.AppendLine("| Benchmark | Base ns | Head ns | Ratio | Limit | Result |")
[void]$markdown.AppendLine("|---|---:|---:|---:|---:|---|")
foreach ($entry in $core) {
    $id = ([string]$entry.id).Replace("|", "\|")
    $state = if ($entry.passed) { "PASS" } else { "FAIL" }
    [void]$markdown.AppendLine("| $id | $($entry.base_median_ns) | $($entry.head_median_ns) | $($entry.ratio) | $criticalLimit | $state |")
}
[void]$markdown.AppendLine()
[void]$markdown.AppendLine("## Absolute budgets")
[void]$markdown.AppendLine()
[void]$markdown.AppendLine("| Benchmark | Median ns | Ratio | Max ratio | Max ns | Result |")
[void]$markdown.AppendLine("|---|---:|---:|---:|---:|---|")
foreach ($entry in $absolute) {
    $id = ([string]$entry.id).Replace("|", "\|")
    $state = if ($entry.passed) { "PASS" } else { "FAIL" }
    [void]$markdown.AppendLine("| $id | $($entry.head_median_ns) | $($entry.ratio) | $($entry.max_ratio) | $($entry.max_median_ns) | $state |")
}
[void]$markdown.AppendLine()
[void]$markdown.AppendLine("## Score ledger")
[void]$markdown.AppendLine()
[void]$markdown.AppendLine("| Condition | Dimension | Points | Status | Awarded |")
[void]$markdown.AppendLine("|---|---|---:|---|---:|")
foreach ($entry in $verified.score_ledger) {
    [void]$markdown.AppendLine("| $($entry.id) | $($entry.dimension) | $($entry.points) | $($entry.status) | $($entry.awarded) |")
}
$markdown.ToString() | Set-Content -Encoding utf8 -LiteralPath $markdownPath

Write-Output (Get-Content -Raw -LiteralPath $jsonPath)
if ([string]$verified.verdict -ne "pass") {
    exit 1
}
