#!/usr/bin/env pwsh
# Bract Build Artifacts Cleanup Script
# This script removes all build artifacts that cause GitHub to detect the repository as 75% "makefile"
# Run this script to clean up your repository before committing

Write-Host " Bract Build Artifacts Cleanup" -ForegroundColor Green
Write-Host "===============================================" -ForegroundColor Green
Write-Host ""

# Function to safely remove directory if it exists
function Remove-IfExists {
    param($Path, $Description)
    if (Test-Path $Path) {
        Write-Host "  Removing $Description..." -ForegroundColor Yellow
        Remove-Item $Path -Recurse -Force
        Write-Host "    $Description removed" -ForegroundColor Green
    } else {
        Write-Host "   $Description not found (already clean)" -ForegroundColor Gray
    }
}

# Function to safely remove file if it exists
function Remove-FileIfExists {
    param($Path, $Description)
    if (Test-Path $Path) {
        Write-Host "  Removing $Description..." -ForegroundColor Yellow
        Remove-Item $Path -Force
        Write-Host "    $Description removed" -ForegroundColor Green
    } else {
        Write-Host "    $Description not found (already clean)" -ForegroundColor Gray
    }
}

Write-Host " Starting cleanup process..." -ForegroundColor Cyan
Write-Host ""

# Remove target directory (Cargo build artifacts)
Remove-IfExists "target" "Cargo target directory"

# Remove test executables and debug files
Remove-FileIfExists "test.exe" "test.exe"
Remove-FileIfExists "test.pdb" "test.pdb" 
Remove-FileIfExists "test.rs" "test.rs"

# Remove any remaining test files
Get-ChildItem -Path "." -Name "test_*.bract" -ErrorAction SilentlyContinue | ForEach-Object {
    Remove-FileIfExists $_ "Test file: $_"
}

# Remove any remaining .exe files in root
Get-ChildItem -Path "." -Name "*.exe" -ErrorAction SilentlyContinue | ForEach-Object {
    Remove-FileIfExists $_ "Executable: $_"
}

# Remove any remaining .pdb files in root
Get-ChildItem -Path "." -Name "*.pdb" -ErrorAction SilentlyContinue | ForEach-Object {
    Remove-FileIfExists $_ "Debug file: $_"
}

# Remove .vs directory if it exists (Visual Studio)
Remove-IfExists ".vs" "Visual Studio cache"

# Remove node_modules if it exists (shouldn't be there but just in case)
Remove-IfExists "node_modules" "Node.js modules"

# Remove any other common build artifacts
Remove-IfExists "build" "Build directory"
Remove-IfExists "dist" "Distribution directory"
Remove-IfExists "out" "Output directory"

Write-Host ""
Write-Host " Cleanup complete!" -ForegroundColor Green
Write-Host ""
Write-Host " Repository Status:" -ForegroundColor Cyan
Write-Host "   • All build artifacts removed" -ForegroundColor Green
Write-Host "   • GitHub language detection will now show correct percentages" -ForegroundColor Green
Write-Host "   • Repository is clean and ready for commit" -ForegroundColor Green
Write-Host ""
Write-Host " Next Steps:" -ForegroundColor Cyan
Write-Host "   1. git add -A" -ForegroundColor White
Write-Host "   2. git commit -m 'Clean up build artifacts'" -ForegroundColor White
Write-Host "   3. git push" -ForegroundColor White
Write-Host ""
Write-Host " Pro Tip: Run this script regularly to keep your repository clean!" -ForegroundColor Yellow 