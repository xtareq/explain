$packageName = 'explain'
$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$installDir = "$(Get-ToolsLocation)\$packageName"

# Create the install directory if it doesn't exist
if (-Not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force
}

# Copy the Rust binary to the install directory
Copy-Item "$toolsDir\target\release\explain.exe" "$installDir\explain.exe" -Force

# Copy and rename the Rust binary to 'exp.exe'
Copy-Item "$toolsDir\target\release\explain.exe" "$installDir\exp.exe" -Force

# Add the install directory to the PATH if it's not already in PATH
if (-Not ($env:Path -contains $installDir)) {
    Install-ChocolateyPath "$installDir" 'Machine'
}
