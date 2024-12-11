$packageName = 'explain'
$installDir = "$(Get-ToolsLocation)\$packageName"

# Remove the install directory if it exists
if (Test-Path $installDir) {
    Remove-Item $installDir -Recurse -Force
}

# Remove the install directory from the system PATH
if ($env:Path -contains $installDir) {
    Uninstall-ChocolateyPath "$installDir" 'Machine'
}
