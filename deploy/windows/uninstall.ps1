#Requires -RunAsAdministrator
<#
.SYNOPSIS
  Uninstall AxleOps Windows service / scheduled task (optionally purge files).

.EXAMPLE
  .\uninstall.ps1 -Component agent
  .\uninstall.ps1 -Component all -Purge -InstallRoot C:\axleops
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true, Position = 0)]
  [ValidateSet("agent", "admin", "proxy", "all")]
  [string]$Component,

  [string]$InstallRoot = "C:\axleops",

  [string]$NssmPath = "",

  [switch]$Purge
)

$ErrorActionPreference = "Stop"

function Find-Nssm {
  if ($NssmPath -and (Test-Path $NssmPath)) { return (Resolve-Path $NssmPath).Path }
  $cmd = Get-Command nssm -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }
  foreach ($p in @(
      "C:\ProgramData\chocolatey\bin\nssm.exe",
      "C:\Tools\nssm\nssm.exe",
      (Join-Path $PSScriptRoot "nssm.exe")
    )) {
    if (Test-Path $p) { return $p }
  }
  return $null
}

function Remove-One([string]$Name) {
  $svc = "AxleOps-$Name"
  $task = "AxleOps-$Name"
  Write-Host "==> uninstall $Name"

  $nssm = Find-Nssm
  $existing = Get-Service -Name $svc -ErrorAction SilentlyContinue
  if ($existing) {
    if ($nssm) {
      & $nssm stop $svc 2>$null | Out-Null
      & $nssm remove $svc confirm 2>$null | Out-Null
    } else {
      Stop-Service -Name $svc -Force -ErrorAction SilentlyContinue
      sc.exe delete $svc | Out-Null
    }
  }

  Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue

  if ($Purge) {
    $dir = Join-Path $InstallRoot $Name
    if (Test-Path $dir) {
      Write-Host "    purge $dir"
      Remove-Item -Recurse -Force $dir
    }
  }
}

$names = if ($Component -eq "all") { @("agent", "admin", "proxy") } else { @($Component) }
foreach ($n in $names) { Remove-One $n }
Write-Host "done."
