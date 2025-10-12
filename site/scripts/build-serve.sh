#!/bin/bash
set -euo pipefail

PORT=${PORT:-3000}

# Build the site
echo "Building static site..."
pnpm build

# Check if build succeeded
if [ ! -d "out" ]; then
    echo "Error: Build failed. The 'out' directory was not created."
    exit 1
fi

# Serve the site
echo "Starting server on port $PORT..."
cd out && python3 -m http.server $PORT