#!/bin/bash

# Clear JavaScript-related cache folders recursively
# Usage: ./clear_js_cache.sh [directory]

# Set the target directory (default to current directory)
TARGET_DIR="${1:-.}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Clearing JavaScript cache folders in: $TARGET_DIR${NC}"

# Counter for removed folders
removed_count=0

# Function to safely remove directory
remove_dir() {
    local dir="$1"
    if [ -d "$dir" ]; then
        echo -e "Removing: ${RED}$dir${NC}"
        rm -rf "$dir"
        ((removed_count++))
    fi
}

# Find and remove cache folders
while IFS= read -r -d '' dir; do
    remove_dir "$dir"
done < <(find "$TARGET_DIR" -type d \( \
    -name "node_modules" -o \
    -name ".next" -o \
    -name ".nuxt" -o \
    -name "dist" -o \
    -name "build" -o \
    -name ".cache" -o \
    -name ".parcel-cache" -o \
    -name ".webpack" -o \
    -name ".rollup.cache" -o \
    -name ".vite" -o \
    -name ".turbo" -o \
    -name ".nx" -o \
    -name "coverage" -o \
    -name ".nyc_output" -o \
    -name "out" -o \
    -name ".output" -o \
    -name ".vercel" -o \
    -name ".netlify" -o \
    -name "storybook-static" \
\) -print0)

# Also remove common cache files
echo -e "${YELLOW}Removing cache files...${NC}"
find "$TARGET_DIR" -type f \( \
    -name "*.log" -o \
    -name ".DS_Store" -o \
    -name "Thumbs.db" -o \
    -name "*.tmp" -o \
    -name "*.temp" \
\) -delete 2>/dev/null

# Remove package-lock.json and yarn.lock (optional - uncomment if needed)
# find "$TARGET_DIR" -name "package-lock.json" -delete 2>/dev/null
# find "$TARGET_DIR" -name "yarn.lock" -delete 2>/dev/null

echo -e "${GREEN}✓ Cleanup complete! Removed $removed_count cache folders.${NC}"

# Optional: Show disk space freed (requires du command)
if command -v du >/dev/null 2>&1; then
    echo -e "${YELLOW}Disk space check completed.${NC}"
fi
