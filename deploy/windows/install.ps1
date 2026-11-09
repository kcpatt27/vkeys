#Requires -Version 7.0
<#
.SYNOPSIS
  Install the hardened vkeys broker on Windows as a SYSTEM scheduled task.

.DESCRIPTION
  The broker runs as SYSTEM. Its vault, master key, and profiles live in
  C:\ProgramData\vkeys-broker, ACLed to SYSTEM + Administrators only. The agent
  user cannot read them; it only gets an AppData broker token (use-only).

  Run in an elevated PowerShell (Run as administrator).

.EXAMPLE
  pwsh -File .\deploy\windows\install.ps1

.EXAMPLE
  $env:VKEYS_PROVIDER_KEY = 'sk-or-v1-...'
  pwsh -File .\deploy\windows\install.ps1 -NonInteractive
#>
[CmdletBinding()]
param(
    [string]$BinDir = (Join-Path $PSScriptRoot "..\..\rust\target\release"),
    [string]$AgentTokenPath = (Join-Path $env:APPDATA "vkeys\broker.token"),
    [string]$AgentUser = ((& whoami).Trim()),
    [int]$Port = 8787,
    [string]$ProviderKeyFile,
    [switch]$NonInteractive,
    [switch]$Uninstall,
    [switch]$RemoveData
)

$ErrorActionPreference = "Stop"
$TaskName = "vkeys-broker"
$InstallDir = Join-Path $env:ProgramFiles "vkeys"
$DataDir = Join-Path $env:ProgramData "vkeys-broker"
$Vault = Join-Path $DataDir "vault.bin"
$KeyFile = Join-Path $DataDir "master.key"
$Profiles = Join-Path $DataDir "profiles.toml"
$Runner = Join-Path $DataDir "run.cmd"
$Vkeys = Join-Path $InstallDir "vkeys.exe"
$Broker = Join-Path $InstallDir "vkeys-broker.exe"

function Test-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    return (New-Object Security.Principal.WindowsPrincipal($id)).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator
    )
}

function Set-LockedAcl([string]$Path) {
    $isDir = Test-Path -LiteralPath $Path -PathType Container
    $grant = if ($isDir) { "(OI)(CI)F" } else { "F" }
    & icacls $Path /inheritance:r /grant:r "*S-1-5-18:$grant" "*S-1-5-32-544:$grant" 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        # A previous run may have left a file owned by another identity.
        & takeown /f $Path /a 2>&1 | Out-Null
        & icacls $Path /inheritance:r /grant:r "*S-1-5-18:$grant" "*S-1-5-32-544:$grant" 2>&1 | Out-Null
    }
    if ($LASTEXITCODE -ne 0) { throw "cannot set ACL on $Path" }
}

if (-not (Test-Admin)) {
    throw "Run this in an elevated PowerShell (Run as administrator)."
}

if ($Uninstall) {
    Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
    Write-Host "removed scheduled task $TaskName"
    if ($RemoveData) {
        Remove-Item -Recurse -Force $DataDir -ErrorAction SilentlyContinue
        Remove-Item -Recurse -Force $InstallDir -ErrorAction SilentlyContinue
        Remove-Item -Force $AgentTokenPath -ErrorAction SilentlyContinue
        Write-Host "removed $DataDir, $InstallDir, and $AgentTokenPath"
    }
    return
}

if (-not (Test-Path (Join-Path $BinDir "vkeys.exe")) -or
    -not (Test-Path (Join-Path $BinDir "vkeys-broker.exe"))) {
    throw "vkeys.exe / vkeys-broker.exe not found in $BinDir"
}

New-Item -ItemType Directory -Force $InstallDir | Out-Null
New-Item -ItemType Directory -Force $DataDir | Out-Null
Set-LockedAcl $DataDir

# Earlier builds did not mark the directory ACEs inheritable, so a vault.tmp
# written by another identity could replace vault.bin with an ACL that locks
# out Administrators. Repair any existing files before reading the vault.
foreach ($file in @($KeyFile, $Vault, $Profiles, (Join-Path $DataDir "vault.tmp"))) {
    if (Test-Path -LiteralPath $file) { Set-LockedAcl $file }
}

Copy-Item (Join-Path $BinDir "vkeys.exe") $Vkeys -Force
Copy-Item (Join-Path $BinDir "vkeys-broker.exe") $Broker -Force

# profiles: paths are TOML strings, so double the backslashes.
$brokerToml = $Broker -replace '\\', '\\\\'
$profileText = @"
[profile.openrouter-broker]
cmd = ["$brokerToml", "--host", "127.0.0.1", "--port", "$Port"]
agent-ok = true

[profile.openrouter-broker.keys]
OPENROUTER_API_KEY = "openrouter/broker"
VKEYS_BROKER_TOKEN = "broker/token"
"@
Set-Content -Path $Profiles -Value $profileText -Encoding ascii

$env:VKEYS_VAULT = $Vault
$env:VKEYS_MASTER_KEY_FILE = $KeyFile
$env:VKEYS_PROFILES = $Profiles
$env:VKEYS_SCHEME = "xchacha20poly1305"

$listing = & $Vkeys ls 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "cannot read the vault at $Vault; run this installer elevated and check the ACL on $DataDir"
}
$hasProvider = ($listing | Where-Object { $_ -match '^openrouter/broker\s' }).Count -gt 0
$hasToken = ($listing | Where-Object { $_ -match '^broker/token\s' }).Count -gt 0

if (-not $hasProvider) {
    $provider = $null
    if ($ProviderKeyFile) {
        $provider = (Get-Content -Raw -Path $ProviderKeyFile).Trim()
    } elseif ($NonInteractive -and $env:VKEYS_PROVIDER_KEY) {
        $provider = $env:VKEYS_PROVIDER_KEY.Trim()
    } else {
        $secure = Read-Host "OpenRouter provider key (hidden)" -AsSecureString
        $ptr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
        try { $provider = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($ptr).Trim() }
        finally { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($ptr) }
    }
    if (-not $provider) { throw "no provider key entered" }
    $provider | & $Vkeys set openrouter/broker --stdin
    if ($LASTEXITCODE -ne 0) { throw "failed to store the provider key" }
    Remove-Variable provider, secure -ErrorAction SilentlyContinue
    $hasProvider = $true
}

if (-not ($hasToken -and (Test-Path $AgentTokenPath))) {
    $bytes = New-Object byte[] 32
    [Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
    $token = -join ($bytes | ForEach-Object { $_.ToString("x2") })
    $token | & $Vkeys set broker/token --stdin
    if ($LASTEXITCODE -ne 0) { throw "failed to store the broker token" }
    New-Item -ItemType Directory -Force (Split-Path $AgentTokenPath) | Out-Null
    Set-Content -Path $AgentTokenPath -Value $token -NoNewline -Encoding ascii
    & icacls $AgentTokenPath /inheritance:r /grant:r "${AgentUser}:(R)" "*S-1-5-18:(F)" "*S-1-5-32-544:(F)" | Out-Null
    Remove-Variable token, bytes -ErrorAction SilentlyContinue
    Write-Host "broker token written to $AgentTokenPath"
}

foreach ($file in @($KeyFile, $Vault, $Profiles)) {
    if (Test-Path $file) { Set-LockedAcl $file }
}

$runnerText = @"
@echo off
set "VKEYS_VAULT=$Vault"
set "VKEYS_MASTER_KEY_FILE=$KeyFile"
set "VKEYS_PROFILES=$Profiles"
set "VKEYS_SCHEME=xchacha20poly1305"
:loop
"$Vkeys" run openrouter-broker
timeout /t 3 /nobreak >nul
goto loop
"@
Set-Content -Path $Runner -Value $runnerText -Encoding ascii
Set-LockedAcl $Runner

Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
$action = New-ScheduledTaskAction -Execute $Runner
$trigger = New-ScheduledTaskTrigger -AtStartup
$principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest
$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -RestartCount 999 `
    -RestartInterval (New-TimeSpan -Minutes 1) `
    -ExecutionTimeLimit (New-TimeSpan -Days 3650) `
    -MultipleInstances IgnoreNew
Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger `
    -Principal $principal -Settings $settings -Force | Out-Null
Start-ScheduledTask -TaskName $TaskName

$healthy = $null
for ($i = 0; $i -lt 20; $i++) {
    try {
        $healthy = Invoke-RestMethod "http://127.0.0.1:$Port/healthz" -TimeoutSec 2
        break
    } catch {
        Start-Sleep -Seconds 1
    }
}

if (-not $healthy) {
    Write-Host "health check failed; task info:" -ForegroundColor Red
    Get-ScheduledTaskInfo -TaskName $TaskName | Format-List
    throw "broker did not become healthy on port $Port"
}

Write-Host ""
Write-Host "vkeys broker healthy: $($healthy | ConvertTo-Json -Compress)"
Write-Host "  endpoint:    http://127.0.0.1:$Port/v1"
Write-Host "  agent token: $AgentTokenPath (use-only, ACL: $AgentUser read)"
Write-Host "  secrets dir: $DataDir (SYSTEM + Administrators only)"
Write-Host ""
Write-Host "Verify the boundary:"
Write-Host "  icacls `"$KeyFile`""
Write-Host "  # as the agent user, this must fail:"
Write-Host "  Get-Content `"$KeyFile`""
