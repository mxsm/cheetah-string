param(
    [string]$Msrv = "1.95",
    [switch]$AllowDirty
)

$ErrorActionPreference = "Stop"

function Invoke-Cargo {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)

    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "cargo $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

$metadata = (& cargo metadata --no-deps --format-version 1 | ConvertFrom-Json)
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed"
}
$version = $metadata.packages[0].version

$packageArguments = @("package", "--no-verify")
if ($AllowDirty) {
    $packageArguments += "--allow-dirty"
}
Invoke-Cargo @packageArguments
$crateArchive = Join-Path (Get-Location) "target/package/cheetah-string-$version.crate"
if (-not (Test-Path -LiteralPath $crateArchive -PathType Leaf)) {
    throw "package archive is missing: $crateArchive"
}
$crateArchive = (Resolve-Path -LiteralPath $crateArchive).Path

$temporaryRoot = Join-Path ([IO.Path]::GetTempPath()) ("cheetah-string-msrv-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
try {
& tar -xzf $crateArchive -C $temporaryRoot
if ($LASTEXITCODE -ne 0) {
    throw "failed to unpack package archive"
}
$packageDir = Join-Path $temporaryRoot "cheetah-string-$version"
if (-not (Test-Path -LiteralPath $packageDir -PathType Container)) {
    throw "unpacked package directory is missing: $packageDir"
}
$packageDir = (Resolve-Path -LiteralPath $packageDir).Path

function Test-Consumer {
    param(
        [string]$Name,
        [string]$Features
    )

    $consumer = Join-Path $temporaryRoot "consumer-$Name"
    Invoke-Cargo "+$Msrv" new --lib --name $Name $consumer
    $addArguments = @(
        "+$Msrv", "add",
        "--manifest-path", (Join-Path $consumer "Cargo.toml"),
        "--path", $packageDir,
        "--no-default-features"
    )
    if ($Features) {
        $addArguments += @("--features", $Features)
    }
    Invoke-Cargo @addArguments
    Invoke-Cargo "+$Msrv" check --manifest-path (Join-Path $consumer "Cargo.toml")
}

    Test-Consumer "cheetah_msrv_no_default" ""
    Test-Consumer "cheetah_msrv_all_features" "std,serde,bytes,experimental-packed,experimental-simd,simd"
    Test-Consumer "cheetah_msrv_optional_no_std" "serde,bytes"
}
finally {
    $resolvedTemporaryRoot = (Resolve-Path -LiteralPath $temporaryRoot).Path
    $expectedPrefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    if (-not $resolvedTemporaryRoot.StartsWith($expectedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "temporary MSRV directory escaped the system temporary directory"
    }
    Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force
}
