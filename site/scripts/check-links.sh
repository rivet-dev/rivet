#!/bin/bash
set -euo pipefail

URL=${1:-http://localhost:3000}
LOG_FILE="/tmp/wget-link-check.log"

# Check if wget is installed
if ! command -v wget &> /dev/null; then
    echo "Error: wget is not installed."
    echo "Install with: brew install wget"
    exit 1
fi

# Check if server is running
echo "Checking if server is running at $URL..."
if ! curl -s -o /dev/null "$URL"; then
    echo "Error: Server is not responding at $URL"
    echo "Run: ./scripts/build-serve.sh"
    exit 1
fi
echo "Server is ready."

# Run wget spider to check for broken links
echo "Checking for broken links (this may take a minute)..."
echo "Debug output: $LOG_FILE"
wget --spider --recursive --no-parent --level=5 --reject="*.js,*.css,*.png,*.jpg,*.svg,*.woff,*.woff2" "$URL"

