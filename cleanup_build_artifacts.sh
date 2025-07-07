#!/bin/bash
# Bract Build Artifacts Cleanup Script (Unix/Linux/Mac version)
# This script removes all build artifacts that cause GitHub to detect the repository as 75% "makefile"
# Run this script to clean up your repository before committing

echo " Bract Build Artifacts Cleanup"
echo "==============================================="
echo ""

# Function to safely remove directory if it exists
remove_dir_if_exists() {
    if [ -d "$1" ]; then
        echo "  Removing $2..."
        rm -rf "$1"
        echo "    $2 removed"
    else
        echo "    $2 not found (already clean)"
    fi
}

# Function to safely remove file if it exists
remove_file_if_exists() {
    if [ -f "$1" ]; then
        echo "  Removing $2..."
        rm -f "$1"
        echo "    $2 removed"
    else
        echo "    $2 not found (already clean)"
    fi
}

echo "🎯 Starting cleanup process..."
echo ""

# Remove target directory (Cargo build artifacts)
remove_dir_if_exists "target" "Cargo target directory"

# Remove test executables and debug files
remove_file_if_exists "test.exe" "test.exe"
remove_file_if_exists "test.pdb" "test.pdb"
remove_file_if_exists "test.rs" "test.rs"

# Remove any remaining test files
for file in test_*.bract; do
    if [ -f "$file" ]; then
        remove_file_if_exists "$file" "Test file: $file"
    fi
done

# Remove any remaining .exe files in root
for file in *.exe; do
    if [ -f "$file" ]; then
        remove_file_if_exists "$file" "Executable: $file"
    fi
done

# Remove any remaining .pdb files in root
for file in *.pdb; do
    if [ -f "$file" ]; then
        remove_file_if_exists "$file" "Debug file: $file"
    fi
done

# Remove common build artifacts
remove_dir_if_exists ".vs" "Visual Studio cache"
remove_dir_if_exists "node_modules" "Node.js modules"
remove_dir_if_exists "build" "Build directory"
remove_dir_if_exists "dist" "Distribution directory"
remove_dir_if_exists "out" "Output directory"

echo ""
echo " Cleanup complete!"
echo ""
echo " Repository Status:"
echo "   • All build artifacts removed"
echo "   • GitHub language detection will now show correct percentages"
echo "   • Repository is clean and ready for commit"
echo ""
echo " Next Steps:"
echo "   1. git add -A"
echo "   2. git commit -m 'Clean up build artifacts'"
echo "   3. git push"
echo ""
echo " Pro Tip: Run this script regularly to keep your repository clean!" 