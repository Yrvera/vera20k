[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Worktree
)

$ErrorActionPreference = "Stop"

function ConvertTo-NormalizedPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    return [System.IO.Path]::GetFullPath($Path).TrimEnd([char]92, [char]47)
}

$resolvedWorktree = (Resolve-Path -LiteralPath $Worktree).Path
$worktreeRoot = ConvertTo-NormalizedPath $resolvedWorktree
if (-not (Get-Item -LiteralPath $worktreeRoot).PSIsContainer) {
    throw "Worktree path is not a directory: $worktreeRoot"
}

$gitRootOutput = & git -C $worktreeRoot rev-parse --show-toplevel 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "Not a Git worktree: $worktreeRoot"
}
$gitRoot = ConvertTo-NormalizedPath ($gitRootOutput | Select-Object -First 1)
if (-not $gitRoot.Equals($worktreeRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Pass the worktree root, not a subdirectory: $gitRoot"
}

$externalLinks = @()
$reparsePoints = @(Get-ChildItem -LiteralPath $worktreeRoot -Force -Recurse `
    -Attributes ReparsePoint -ErrorAction Stop)
foreach ($link in $reparsePoints) {
    $targets = @($link.Target)
    if ($targets.Count -eq 0 -or [string]::IsNullOrWhiteSpace([string]$targets[0])) {
        $externalLinks += $link.FullName
        Write-Output "reparse=$($link.FullName)|target=UNKNOWN|inside_worktree=False"
        continue
    }

    foreach ($target in $targets) {
        $targetPath = if ([System.IO.Path]::IsPathRooted($target)) {
            ConvertTo-NormalizedPath $target
        } else {
            ConvertTo-NormalizedPath (Join-Path $link.DirectoryName $target)
        }
        $insideWorktree = $targetPath.Equals(
            $worktreeRoot,
            [System.StringComparison]::OrdinalIgnoreCase
        ) -or $targetPath.StartsWith(
            "$worktreeRoot\",
            [System.StringComparison]::OrdinalIgnoreCase
        )
        Write-Output "reparse=$($link.FullName)|target=$targetPath|inside_worktree=$insideWorktree"
        if (-not $insideWorktree) {
            $externalLinks += $link.FullName
        }
    }
}

if ($externalLinks.Count -gt 0) {
    throw "Unsafe external reparse point(s): $($externalLinks -join ', ')"
}

$dirty = @(& git -C $worktreeRoot status --porcelain=v1 --untracked-files=all)
if ($LASTEXITCODE -ne 0) {
    throw "Unable to inspect worktree status: $worktreeRoot"
}
if ($dirty.Count -gt 0) {
    $preview = ($dirty | Select-Object -First 20) -join [Environment]::NewLine
    throw "Worktree has tracked or untracked changes:`n$preview"
}

$ignored = @(& git -C $worktreeRoot clean -ndX)
if ($LASTEXITCODE -ne 0) {
    throw "Unable to preview ignored-file cleanup: $worktreeRoot"
}

Write-Output "worktree=$worktreeRoot"
Write-Output "external_reparse_points=0"
Write-Output "ignored_cleanup_candidates=$($ignored.Count)"
$ignored | Select-Object -First 50
if ($ignored.Count -gt 50) {
    Write-Output "ignored_cleanup_candidates_omitted=$($ignored.Count - 50)"
}
