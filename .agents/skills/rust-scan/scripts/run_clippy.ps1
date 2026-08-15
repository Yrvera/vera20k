[CmdletBinding()]
param(
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

$activeBuilds = @(
    Get-Process cargo, rustc -ErrorAction SilentlyContinue |
        Select-Object -Property Id, ProcessName, StartTime
)

if ($activeBuilds.Count -gt 0) {
    Write-Error (
        'Cargo or rustc is already active; rust-scan will not compete with another build. ' +
        ($activeBuilds | Format-Table -AutoSize | Out-String).Trim()
    )
    exit 3
}

$repoRoot = (& git rev-parse --show-toplevel 2>$null).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    Write-Error 'run_clippy.ps1 must run from inside the VERA20k repository.'
    exit 2
}

$manifestPath = Join-Path -Path $repoRoot -ChildPath 'Cargo.toml'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    Write-Error "Cargo.toml was not found at repository root: $repoRoot"
    exit 2
}

$cargoArgs = @(
    'clippy'
    '-p'
    'vera20k'
    '--lib'
    '--no-deps'
    '--locked'
    '--message-format=short'
    '--'
    '-A'
    'clippy::all'
    '-W'
    'clippy::correctness'
    '-W'
    'clippy::suspicious'
    '-W'
    'clippy::perf'
    '-W'
    'clippy::undocumented_unsafe_blocks'
    '-W'
    'clippy::missing_safety_doc'
    '-W'
    'unsafe_op_in_unsafe_fn'
)

Write-Output ('cargo ' + ($cargoArgs -join ' '))
if ($DryRun) {
    exit 0
}

Push-Location -LiteralPath $repoRoot
try {
    & cargo @cargoArgs 2>&1
    $cargoExitCode = $LASTEXITCODE
}
finally {
    Pop-Location
}

if ($null -eq $cargoExitCode) {
    Write-Error 'Cargo did not return an exit code.'
    exit 2
}

exit $cargoExitCode
