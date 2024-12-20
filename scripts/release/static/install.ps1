#!/usr/bin/env pwsh

$ErrorActionPreference = 'Stop'

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# Create bin directory for Rivet
$BinDir = $env:BIN_DIR
$RivetInstall = if ($BinDir) {
	$BinDir
} else {
	"${Home}\.rivet\bin"
}

if (!(Test-Path $RivetInstall)) {
	New-Item $RivetInstall -ItemType Directory | Out-Null
}

$RivetExe = "$RivetInstall\rivet.exe"
$Version = '__VERSION__'
$FileName = 'rivet-x86-windows.exe'

Write-Host
Write-Host "> Installing Rivet CLI ${Version}"

# Download CLI
$DownloadUrl = "https://releases.rivet.gg/rivet/${Version}/${FileName}"
Write-Host
Write-Host "> Downloading ${DownloadUrl}"
Invoke-WebRequest $DownloadUrl -OutFile $RivetExe -UseBasicParsing

# Install CLI
Write-Host
Write-Host "> Installing rivet"
$User = [System.EnvironmentVariableTarget]::User
$Path = [System.Environment]::GetEnvironmentVariable('Path', $User)
if (!(";${Path};".ToLower() -like "*;${RivetInstall};*".ToLower())) {
	[System.Environment]::SetEnvironmentVariable('Path', "${Path};${RivetInstall}", $User)
	$Env:Path += ";${RivetInstall}"
    Write-Host "Please restart your PowerShell session or run the following command to refresh the environment variables:"
    Write-Host "[System.Environment]::SetEnvironmentVariable('Path', '${Path};${RivetInstall}', [System.EnvironmentVariableTarget]::Process)"
}

Write-Host
Write-Host "> Checking installation"
rivet.exe --version

Write-Host
Write-Host "Rivet was installed successfully to ${RivetExe}."
Write-Host "Run 'rivet --help' to get started."
Write-Host
