param(
  [string[]]$Names = @(
    "show-routing",
    "route-activate",
    "delegate",
    "delegate-to",
    "task-route",
    "delegate-auto"
  )
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$sourceRoot = Join-Path $repoRoot "skills"
$targetRoot = Join-Path $repoRoot ".claude\skills"

if (-not (Test-Path $sourceRoot)) {
  throw "Skills source folder not found: $sourceRoot"
}

New-Item -ItemType Directory -Force -Path $targetRoot | Out-Null

foreach ($name in $Names) {
  $expandedNames = $name -split "," | ForEach-Object { $_.Trim() } | Where-Object { $_ }
  foreach ($expanded in $expandedNames) {
    $src = Join-Path $sourceRoot $expanded
    if (-not (Test-Path $src)) {
      throw "Skill not found: $src"
    }
    $dst = Join-Path $targetRoot $expanded
    if (Test-Path $dst) {
      Remove-Item -Recurse -Force $dst
    }
    Copy-Item -Recurse -Force $src $dst
    Write-Host "Installed skill: $expanded"
  }
}

Write-Host ""
Write-Host "Installed to: $targetRoot"
Write-Host "Restart Claude Code or reopen the slash menu if needed."
