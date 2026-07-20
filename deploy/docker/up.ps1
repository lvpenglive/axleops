#Requires -Version 5.1
<#
.SYNOPSIS
  One-shot Docker Compose up for AxleOps (Admin + Agent).

.EXAMPLE
  .\up.ps1
  .\up.ps1 -Profile proxy
#>
[CmdletBinding()]
param(
  [string[]]$Profile = @(),
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$ComposeArgs
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $Root

$EnvFile = "deploy\docker\.env"
if (-not (Test-Path $EnvFile)) {
  Copy-Item "deploy\docker\.env.example" $EnvFile
  Write-Host "created $EnvFile — edit secrets, then re-run if needed"
}

New-Item -ItemType Directory -Force -Path "deploy\docker\apps" | Out-Null

$argsList = @("--env-file", $EnvFile, "up", "-d", "--build")
foreach ($p in $Profile) {
  $argsList = @("--profile", $p) + $argsList
}
if ($ComposeArgs) { $argsList += $ComposeArgs }

Write-Host "building & starting…"
& docker compose @argsList
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "Admin UI: http://127.0.0.1:9000"
Write-Host "Register Agent Base URL from Admin container: http://agent:9100"
Write-Host "Token: AGENT_TOKEN in $EnvFile"
& docker compose --env-file $EnvFile ps
