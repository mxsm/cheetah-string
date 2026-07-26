param(
    [Parameter(Mandatory = $true)]
    [string]$Repo,
    [string]$OutFile = "bench-results/usage-inventory/rocketmq-current.json"
)

$ErrorActionPreference = "Stop"
$resolvedRepo = (Resolve-Path -LiteralPath $Repo).Path
$sha = (git -C $resolvedRepo rev-parse HEAD).Trim()
$glob = "**/src/**/*.rs"

function Measure-RgPattern {
    param([string]$Name, [string]$Pattern)

    $matches = @(& rg -o --glob $glob $Pattern $resolvedRepo 2>$null)
    if ($LASTEXITCODE -notin @(0, 1)) {
        throw "rg failed for pattern '$Pattern'"
    }
    $files = @(& rg -l --glob $glob $Pattern $resolvedRepo 2>$null)
    if ($LASTEXITCODE -notin @(0, 1)) {
        throw "rg failed while listing files for pattern '$Pattern'"
    }

    [ordered]@{
        name = $Name
        pattern = $Pattern
        command = "rg -o --glob '$glob' '$Pattern' <repo>"
        occurrences = $matches.Count
        files = $files.Count
    }
}

$commands = @(
    (Measure-RgPattern "CheetahString" "CheetahString"),
    (Measure-RgPattern "CheetahStr" "CheetahStr\b"),
    (Measure-RgPattern "CheetahBuilder" "CheetahBuilder"),
    (Measure-RgPattern "constructors" "CheetahString::(from|from_slice|from_string|from_static_str|try_from_vec|try_from_bytes)\b"),
    (Measure-RgPattern "finish_string" "finish_string\s*\("),
    (Measure-RgPattern "from_string_owned" "from_string_owned\s*\("),
    (Measure-RgPattern "with_capacity" "CheetahString::with_capacity\s*\("),
    (Measure-RgPattern "push_str_lexical" "\.push_str\s*\("),
    (Measure-RgPattern "reserve_lexical" "\.reserve\s*\("),
    (Measure-RgPattern "typed_split" "\.(split_char|split_str)\s*\("),
    (Measure-RgPattern "collection_key" "(HashMap|HashSet)\s*<[^>]*CheetahString"),
    (Measure-RgPattern "trait_bound" "(Into|From|AsRef|Borrow)\s*<\s*CheetahString\s*>"),
    (Measure-RgPattern "public_signature" "pub(\([^)]*\))?\s+(async\s+)?fn[^{;]*CheetahString"),
    (Measure-RgPattern "CheetahBytes" "CheetahBytes")
)

$metadata = cargo metadata --locked --manifest-path (Join-Path $resolvedRepo "Cargo.toml") --no-deps --format-version 1 |
    ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed"
}
$workspaceDependencyPackages = @(
    $metadata.packages | Where-Object {
        $_.dependencies | Where-Object { $_.name -eq "cheetah-string" }
    }
)

$inventory = [ordered]@{
    schema_version = 2
    repository = "rocketmq-rust"
    git_sha = $sha
    generated_at_utc = [DateTime]::UtcNow.ToString("o")
    scope = [ordered]@{
        glob = $glob
        kind = "lexical inventory"
        gitignore_rules_honored = $true
        explicit_exclusions = @()
        includes_comments = $true
        includes_imports = $true
        includes_cfg_test = $true
        not_a_runtime_profile = $true
        not_type_resolved = $true
    }
    commands = $commands
    dependency_packages = [ordered]@{
        workspace = $workspaceDependencyPackages.Count
        standalone = 2
        total = $workspaceDependencyPackages.Count + 2
        workspace_names = @($workspaceDependencyPackages.name | Sort-Object)
        standalone_names = @(
            "rocketmq-example"
            "rocketmq-dashboard-backend"
        )
    }
}

$parent = Split-Path -Parent $OutFile
if ($parent) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}
$inventory | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 -LiteralPath $OutFile
Write-Host "Wrote $OutFile for $sha"
