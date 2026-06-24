param(
    [switch]$RunMiri,
    [switch]$RunSanitizer,
    [switch]$RunFuzz,
    [switch]$RunBench,
    [int]$FuzzRuns = 1000,
    [string]$OutputRoot = "bench-results/packed-evidence"
)

$ErrorActionPreference = "Continue"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$Stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$OutputDir = Join-Path (Join-Path $RepoRoot $OutputRoot) $Stamp
$SummaryPath = Join-Path $OutputDir "summary.md"
$FuzzRunArg = "-runs=$FuzzRuns"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

function Add-SummaryLine {
    param([string]$Line)
    Add-Content -LiteralPath $SummaryPath -Value $Line -Encoding UTF8
}

function Test-CommandAvailable {
    param([string]$Command)
    $null -ne (Get-Command $Command -ErrorAction SilentlyContinue)
}

function Add-AsanRuntimeToPath {
    $Roots = @(
        (Join-Path $env:ProgramFiles "Microsoft Visual Studio"),
        (Join-Path $env:ProgramFiles "LLVM")
    ) | Where-Object { Test-Path -LiteralPath $_ }

    $RuntimeCandidates = foreach ($Root in $Roots) {
        $Matches = Get-ChildItem -LiteralPath $Root -Recurse -Filter "clang_rt.asan_dynamic-x86_64.dll" -ErrorAction SilentlyContinue
        $Matches | Where-Object { $_.FullName -like "*Hostx64*x64*" } | Select-Object -First 1
        $Matches | Select-Object -First 1
    }
    $Runtime = $RuntimeCandidates | Select-Object -First 1

    if ($null -eq $Runtime) {
        Add-SummaryLine "- ``ASan runtime``: not found on PATH or known install roots"
        return
    }

    $RuntimeDir = Split-Path -Parent $Runtime.FullName
    $PathParts = $env:PATH -split ";"
    if ($PathParts -notcontains $RuntimeDir) {
        $env:PATH = "$RuntimeDir;$env:PATH"
    }

    Add-SummaryLine "- ``ASan runtime``: using ``$RuntimeDir``"
}

function Invoke-Gate {
    param(
        [string]$Name,
        [string]$CommandLine,
        [string]$LogName,
        [scriptblock]$Before,
        [scriptblock]$After
    )

    $LogPath = Join-Path $OutputDir $LogName
    $StdoutPath = Join-Path $OutputDir "$LogName.stdout"
    $StderrPath = Join-Path $OutputDir "$LogName.stderr"
    Add-SummaryLine "- ``$Name``: running"
    Push-Location $RepoRoot
    try {
        if ($Before) {
            & $Before
        }
        $Process = Start-Process `
            -FilePath "cmd.exe" `
            -ArgumentList @("/d", "/s", "/c", $CommandLine) `
            -RedirectStandardOutput $StdoutPath `
            -RedirectStandardError $StderrPath `
            -WindowStyle Hidden `
            -Wait `
            -PassThru `
            -ErrorAction Stop
        if ($null -eq $Process) {
            $ExitCode = 1
        } else {
            $ExitCode = $Process.ExitCode
        }
    } catch {
        $_ | Out-File -FilePath $StderrPath -Encoding UTF8
        $ExitCode = 1
    } finally {
        if ($After) {
            & $After
        }
        Pop-Location
    }

    $LogParts = @($StdoutPath, $StderrPath) | Where-Object { Test-Path -LiteralPath $_ }
    if ($LogParts.Count -gt 0) {
        $LogLines = @(Get-Content -LiteralPath $LogParts)
        Set-Content -LiteralPath $LogPath -Encoding UTF8 -Value $LogLines
        $LogLines | Out-Host
        Remove-Item -LiteralPath $LogParts -Force -ErrorAction SilentlyContinue
    } else {
        New-Item -ItemType File -Force -Path $LogPath | Out-Null
    }

    if ($ExitCode -eq 0) {
        Add-SummaryLine "- ``$Name``: PASS, see ``$LogName``"
    } else {
        Add-SummaryLine "- ``$Name``: FAIL exit $ExitCode, see ``$LogName``"
    }

    return [int]$ExitCode
}

Set-Content -LiteralPath $SummaryPath -Encoding UTF8 -Value @(
    "# Packed Evidence Capture",
    "",
    "- Date: $(Get-Date -Format o)",
    "- Repository: cheetah-string",
    "- Output directory: $OutputDir",
    ""
)

$Failed = 0

$Failed += Invoke-Gate `
    -Name "cargo test --features experimental-packed" `
    -LogName "packed-test.txt" `
    -CommandLine "cargo test --features experimental-packed"

if ($RunMiri) {
    $MiriVersion = cargo +nightly miri --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Add-SummaryLine "- ``cargo +nightly miri --version``: $MiriVersion"
        $Failed += Invoke-Gate `
            -Name "cargo +nightly miri test --features experimental-packed packed" `
            -LogName "packed-miri.txt" `
            -CommandLine "cargo +nightly miri test --features experimental-packed packed"
    } else {
        Add-SummaryLine "- ``miri``: SKIP, install with ``rustup component add --toolchain nightly-x86_64-pc-windows-msvc miri``"
    }
} else {
    Add-SummaryLine "- ``miri``: SKIP, pass ``-RunMiri`` to execute"
}

if ($RunSanitizer) {
    Add-AsanRuntimeToPath
    $Failed += Invoke-Gate `
        -Name "address sanitizer packed tests" `
        -LogName "packed-asan.txt" `
        -Before {
            $script:PreviousRustflags = $env:RUSTFLAGS
            $script:HadRustflags = Test-Path Env:RUSTFLAGS
            $env:RUSTFLAGS = "-Z sanitizer=address"
        } `
        -After {
            if ($script:HadRustflags) {
                $env:RUSTFLAGS = $script:PreviousRustflags
            } else {
                Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
            }
        } `
        -CommandLine "cargo +nightly test --features experimental-packed packed"
} else {
    Add-SummaryLine "- ``address sanitizer``: SKIP, pass ``-RunSanitizer`` on a supported nightly target"
}

if ($RunFuzz) {
    if (Test-CommandAvailable "cargo-fuzz") {
        Add-AsanRuntimeToPath
        $Failed += Invoke-Gate `
            -Name "cargo +nightly fuzz run fuzz_packed_from_bytes" `
            -LogName "fuzz-packed-from-bytes.txt" `
            -CommandLine "cargo +nightly fuzz run fuzz_packed_from_bytes -- $FuzzRunArg"
        $Failed += Invoke-Gate `
            -Name "cargo +nightly fuzz run fuzz_packed_push_str" `
            -LogName "fuzz-packed-push-str.txt" `
            -CommandLine "cargo +nightly fuzz run fuzz_packed_push_str -- $FuzzRunArg"
    } else {
        Add-SummaryLine "- ``cargo-fuzz``: SKIP, install with ``cargo install cargo-fuzz``"
    }
} else {
    Add-SummaryLine "- ``cargo fuzz``: SKIP, pass ``-RunFuzz`` to execute"
}

if ($RunBench) {
    $Failed += Invoke-Gate `
        -Name "cargo bench --bench packed --features experimental-packed" `
        -LogName "packed-bench.txt" `
        -CommandLine "cargo bench --bench packed --features experimental-packed"
} else {
    Add-SummaryLine "- ``packed benchmark``: SKIP, pass ``-RunBench`` to execute"
}

Add-SummaryLine ""
Add-SummaryLine "Total failing required gates: $Failed"

if ($Failed -ne 0) {
    exit 1
}
