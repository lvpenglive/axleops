#Requires -RunAsAdministrator
<#
.SYNOPSIS
  Install AxleOps agent / admin / proxy as a Windows service (NSSM) or startup Scheduled Task.

.EXAMPLE
  .\install.ps1 -Component agent -Start
  .\install.ps1 -Component admin -InstallRoot C:\axleops -Start
  .\install.ps1 -Component all -Method ScheduledTask -Start
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true, Position = 0)]
  [ValidateSet("agent", "admin", "proxy", "all")]
  [string]$Component,

  [string]$InstallRoot = "C:\axleops",

  # Package/repo root that contains agent/ or axleops-agent/
  [string]$From = "",

  [ValidateSet("Auto", "Nssm", "ScheduledTask")]
  [string]$Method = "Auto",

  [string]$NssmPath = "",

  [switch]$Start
)

$ErrorActionPreference = "Stop"

function Write-Info([string]$Message) { Write-Host "==> $Message" }

function Get-PackageRoot {
  if ($From) { return (Resolve-Path $From).Path }
  # deploy/windows -> repo or package root
  return (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
}

function Resolve-ComponentPaths([string]$Name) {
  $root = Get-PackageRoot
  $binName = "axleops-$Name.exe"
  $pkgDir = Join-Path $root $Name
  $repoDir = Join-Path $root "axleops-$Name"
  $pkgBin = Join-Path $pkgDir $binName
  $repoBin = Join-Path $repoDir "target\release\$binName"

  if (Test-Path $pkgBin) {
    return @{
      Bin = $pkgBin
      CfgDir = $pkgDir
      Static = Join-Path $pkgDir "static"
    }
  }
  if (Test-Path $repoBin) {
    return @{
      Bin = $repoBin
      CfgDir = $repoDir
      Static = Join-Path $repoDir "static"
    }
  }
  throw "Binary not found for $Name. Looked for:`n  $pkgBin`n  $repoBin"
}

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

function Resolve-Method {
  if ($Method -eq "Nssm") { return "Nssm" }
  if ($Method -eq "ScheduledTask") { return "ScheduledTask" }
  $nssm = Find-Nssm
  if ($nssm) { return "Nssm" }
  Write-Warning "NSSM not found; using Scheduled Task at startup. Install NSSM for a real Windows service."
  return "ScheduledTask"
}

function Install-Files([string]$Name) {
  $paths = Resolve-ComponentPaths $Name
  $dest = Join-Path $InstallRoot $Name
  $binName = "axleops-$Name.exe"
  Write-Info "install $Name -> $dest"
  New-Item -ItemType Directory -Force -Path $dest | Out-Null
  Copy-Item -Force $paths.Bin (Join-Path $dest $binName)

  if ($Name -eq "admin") {
    if (-not (Test-Path $paths.Static)) {
      throw "admin static/ missing: $($paths.Static)"
    }
    $staticDest = Join-Path $dest "static"
    New-Item -ItemType Directory -Force -Path $staticDest | Out-Null
    Copy-Item -Force -Recurse (Join-Path $paths.Static "*") $staticDest
  }

  $cfgDest = Join-Path $dest "config.toml"
  if (-not (Test-Path $cfgDest)) {
    $cfgSrc = Join-Path $paths.CfgDir "config.toml"
    if (-not (Test-Path $cfgSrc)) {
      $cfgSrc = Join-Path $paths.CfgDir "config.example.toml"
    }
    if (-not (Test-Path $cfgSrc)) {
      throw "no config.toml / config.example.toml in $($paths.CfgDir)"
    }
    Copy-Item $cfgSrc $cfgDest
    Write-Host "    wrote $cfgDest from example — edit secrets before start"
  } else {
    Write-Host "    keep existing $cfgDest"
  }

  New-Item -ItemType Directory -Force -Path (Join-Path $dest "data") | Out-Null
  return $dest
}

function Install-WithNssm([string]$Name, [string]$Dest) {
  $nssm = Find-Nssm
  if (-not $nssm) { throw "NSSM not found. Pass -NssmPath or install nssm." }
  $svc = "AxleOps-$Name"
  $exe = Join-Path $Dest "axleops-$Name.exe"
  Write-Info "NSSM service $svc"

  $existing = Get-Service -Name $svc -ErrorAction SilentlyContinue
  if ($existing) {
    & $nssm stop $svc | Out-Null
    & $nssm remove $svc confirm | Out-Null
  }

  & $nssm install $svc $exe
  & $nssm set $svc AppDirectory $Dest
  & $nssm set $svc DisplayName "AxleOps $Name"
  & $nssm set $svc Description "AxleOps $Name process"
  & $nssm set $svc Start SERVICE_AUTO_START
  & $nssm set $svc AppStdout (Join-Path $Dest "service-stdout.log")
  & $nssm set $svc AppStderr (Join-Path $Dest "service-stderr.log")
  & $nssm set $svc AppRotateFiles 1
  & $nssm set $svc AppRotateBytes 10485760

  if ($Start) {
    & $nssm start $svc
    Get-Service $svc | Format-Table -AutoSize
  } else {
    Write-Host "    installed; start with: nssm start $svc   or   Start-Service $svc"
  }
}

function Install-WithTask([string]$Name, [string]$Dest) {
  $taskName = "AxleOps-$Name"
  $exe = Join-Path $Dest "axleops-$Name.exe"
  Write-Info "Scheduled Task $taskName"

  Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue

  $action = New-ScheduledTaskAction -Execute $exe -WorkingDirectory $Dest
  $trigger = New-ScheduledTaskTrigger -AtStartup
  $settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -RestartCount 3 `
    -RestartInterval (New-TimeSpan -Minutes 1)
  $principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest

  Register-ScheduledTask `
    -TaskName $taskName `
    -Action $action `
    -Trigger $trigger `
    -Settings $settings `
    -Principal $principal `
    -Description "AxleOps $Name (startup task)" | Out-Null

  if ($Start) {
    Start-ScheduledTask -TaskName $taskName
    Get-ScheduledTask -TaskName $taskName | Format-Table TaskName, State -AutoSize
  } else {
    Write-Host "    registered; start with: Start-ScheduledTask -TaskName $taskName"
  }
}

function Install-One([string]$Name, [string]$UseMethod) {
  $dest = Install-Files $Name
  if ($UseMethod -eq "Nssm") {
    Install-WithNssm $Name $dest
  } else {
    Install-WithTask $Name $dest
  }
}

$use = Resolve-Method
Write-Info "method=$use installRoot=$InstallRoot from=$(Get-PackageRoot)"

$names = if ($Component -eq "all") { @("agent", "admin", "proxy") } else { @($Component) }
foreach ($n in $names) {
  Install-One $n $use
}

Write-Host "done."
if ($names -contains "agent") {
  Write-Host "note: agent must be able to read/exec managed JAR/script paths."
}
