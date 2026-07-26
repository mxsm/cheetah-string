param(
    [string]$Version = "current",
    [ValidateRange(10, 100000)]
    [int]$SampleCount = 100,
    [ValidateSet("default")]
    [string]$Features = "default",
    [string]$Toolchain = "",
    [switch]$Smoke
)

$ErrorActionPreference = "Stop"
$ResultDir = Join-Path "bench-results" $Version
$CriterionDestination = Join-Path $ResultDir "criterion"
$TargetRoot = if ($env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR
} else {
    Join-Path (Get-Location) "target"
}
$CriterionSource = Join-Path $TargetRoot "criterion"
$cargoPrefix = @()
$rustcPrefix = @()
if ($Toolchain) {
    $cargoPrefix = @("+$Toolchain")
    $rustcPrefix = @("+$Toolchain")
}

$gitSha = (git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "git rev-parse HEAD failed"
}
$gitDirty = @(& git status --porcelain).Count -gt 0

function Get-GitObjectOrAbsent {
    param([string]$RepositoryPath)

    & git cat-file -e "$gitSha`:$RepositoryPath" 2>$null
    if ($LASTEXITCODE -ne 0) {
        return "absent"
    }
    $objectId = (& git rev-parse "$gitSha`:$RepositoryPath").Trim()
    if ($LASTEXITCODE -ne 0 -or -not $objectId) {
        throw "git object lookup failed: $RepositoryPath"
    }
    return $objectId
}

$harnessIdentity = [ordered]@{
    benchmark_tree = Get-GitObjectOrAbsent "benches"
    allocation_contract_blob = Get-GitObjectOrAbsent "tests/allocation_contract.rs"
    layout_contract_blob = Get-GitObjectOrAbsent "tests/layout_snapshot.rs"
    cargo_toml_blob = Get-GitObjectOrAbsent "Cargo.toml"
    cargo_config_tree = Get-GitObjectOrAbsent ".cargo"
    build_script_blob = Get-GitObjectOrAbsent "build.rs"
}
$runtimeTree = Get-GitObjectOrAbsent "src"

if (Test-Path -LiteralPath $ResultDir) {
    throw "benchmark capture already exists: $ResultDir"
}
if (Test-Path -LiteralPath $CriterionSource) {
    throw "Criterion output already exists; use a fresh CARGO_TARGET_DIR: $CriterionSource"
}
New-Item -ItemType Directory -Path $ResultDir | Out-Null

$rustcVerbose = (& rustc @rustcPrefix -vV) -join "`n"
if ($LASTEXITCODE -ne 0) {
    throw "rustc -vV failed"
}
$rustcLine = ($rustcVerbose -split "`n" | Where-Object { $_ -like "rustc *" } | Select-Object -First 1)
if (-not $rustcLine) {
    $rustcLine = (& rustc @rustcPrefix --version).Trim()
}
$llvm = (($rustcVerbose -split "`n" | Where-Object { $_ -like "LLVM version:*" } | Select-Object -First 1) -replace "^LLVM version:\s*", "").Trim()
$cpu = if ($IsWindows -or $env:OS -eq "Windows_NT") {
    (Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name).Trim()
} else {
    [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
}
$os = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
$effectiveFeatures = "$Features;simd-bench=isolated"

$benchmarkIds = @(
    "layout",
    "comprehensive",
    "mutation",
    "mq_topic",
    "mq_properties",
    "mq_remoting_header",
    "pattern",
    "simd",
    "shared_backing"
)

[ordered]@{
    schema_version = 1
    capture_schema_version = "cheetah-string-capture-v2"
    benchmark_schema_version = "cheetah-string-bench-v1"
    criterion_schema_version = "criterion-0.5"
    crate = "cheetah-string"
    git_sha = $gitSha
    runtime_tree = $runtimeTree
    git_dirty = $gitDirty
    rustc = $rustcLine
    llvm = $llvm
    cpu = $cpu
    os = $os
    features = $effectiveFeatures
    simd_feature_alias = "experimental-simd"
    harness_identity = $harnessIdentity
    sample_count = $SampleCount
    smoke = [bool]$Smoke
    profile = "bench"
    generated_at_utc = [DateTime]::UtcNow.ToString("o")
    benchmark_ids = $benchmarkIds
} | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $ResultDir "metadata.json")

function Invoke-CargoCapture {
    param(
        [string]$Output,
        [string[]]$Arguments
    )

    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & cargo @cargoPrefix @Arguments 2>&1 |
        Tee-Object -FilePath (Join-Path $ResultDir $Output)
    $cargoExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousPreference
    if ($cargoExitCode -ne 0) {
        throw "cargo $($Arguments -join ' ') failed"
    }
}

function Assert-TestExecuted {
    param([string]$Output)

    $path = Join-Path $ResultDir $Output
    if (-not (Select-String -LiteralPath $path -Pattern "test result: ok\. [1-9][0-9]* passed; 0 failed" -Quiet)) {
        throw "contract command did not execute a passing test: $path"
    }
}

$criterionArguments = @("--sample-size", "$SampleCount")
if ($Smoke) {
    $criterionArguments += @("--warm-up-time", "0.05", "--measurement-time", "0.10")
}

Invoke-CargoCapture "layout-test.txt" @(
    "test", "--test", "layout_snapshot", "--all-features", "--", "--nocapture"
)
Assert-TestExecuted "layout-test.txt"
Invoke-CargoCapture "allocation-contract.txt" @(
    "test", "--test", "allocation_contract", "--all-features", "--", "--test-threads=1"
)
Assert-TestExecuted "allocation-contract.txt"
[ordered]@{
    schema_version = 1
    layout_contract = "passed"
    allocation_contract = "passed"
    clone_allocations_max = 0
    source = "tests/allocation_contract.rs"
} | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $ResultDir "contracts.json")
Invoke-CargoCapture "layout-bench.txt" (@(
    "bench", "--bench", "layout", "--"
) + $criterionArguments)
Invoke-CargoCapture "comprehensive.txt" (@(
    "bench", "--bench", "comprehensive", "--"
) + $criterionArguments)
Invoke-CargoCapture "mutation.txt" (@(
    "bench", "--bench", "mutation", "--"
) + $criterionArguments)
Invoke-CargoCapture "mq-topic.txt" (@(
    "bench", "--bench", "mq_topic", "--"
) + $criterionArguments)
Invoke-CargoCapture "mq-properties.txt" (@(
    "bench", "--bench", "mq_properties", "--"
) + $criterionArguments)
Invoke-CargoCapture "mq-remoting-header.txt" (@(
    "bench", "--bench", "mq_remoting_header", "--"
) + $criterionArguments)
Invoke-CargoCapture "pattern.txt" (@(
    "bench", "--bench", "pattern", "--"
) + $criterionArguments)
Invoke-CargoCapture "simd.txt" (@(
    "bench", "--bench", "simd", "--features", "experimental-simd", "--"
) + $criterionArguments)
Invoke-CargoCapture "shared-backing.txt" (@(
    "bench", "--bench", "shared_backing", "--"
) + $criterionArguments)

if (-not (Test-Path -LiteralPath $CriterionSource)) {
    throw "Criterion result directory is missing: $CriterionSource"
}
Copy-Item -LiteralPath $CriterionSource -Destination $CriterionDestination -Recurse
