$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$windowsRoot = Join-Path $repoRoot "windows"

$bad = @()
$files = Get-ChildItem -Path $windowsRoot -Recurse -Include "*.csproj", "*.props", "*.targets" -File
if ($files.Count -eq 0) {
    throw "No Windows project files found under $windowsRoot"
}

foreach ($file in $files) {
    [xml]$xml = Get-Content -Raw -Path $file.FullName

    foreach ($node in $xml.SelectNodes("//PackageReference[@Version] | //PackageVersion[@Version]")) {
        $version = $node.GetAttribute("Version")
        if ($version -match "\*") {
            $relativePath = Resolve-Path -Relative -Path $file.FullName
            $bad += "$relativePath`: $($node.GetAttribute("Include")) $version"
        }
    }

    foreach ($node in $xml.SelectNodes("//PackageReference[Version] | //PackageVersion[Version]")) {
        $versionNode = $node.SelectSingleNode("Version")
        if ($null -ne $versionNode -and $versionNode.InnerText -match "\*") {
            $relativePath = Resolve-Path -Relative -Path $file.FullName
            $bad += "$relativePath`: $($node.GetAttribute("Include")) $($versionNode.InnerText)"
        }
    }
}

if ($bad.Count -gt 0) {
    $bad | ForEach-Object { Write-Error $_ }
    exit 1
}
