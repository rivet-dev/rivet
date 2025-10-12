#!/bin/bash
set -euo pipefail

URL=${1:-http://localhost:3000}

# Check if lychee is installed
if ! command -v lychee &> /dev/null; then
    echo "Error: lychee is not installed."
    echo "Install with: brew install lychee"
    exit 1
fi

# Wait for server to be ready
echo "Checking if server is running at $URL..."
for i in {1..10}; do
    if curl -s -o /dev/null "$URL"; then
        echo "Server is ready."
        break
    fi
    if [ $i -eq 10 ]; then
        echo "Error: Server is not responding at $URL"
        echo "Run: ./scripts/build-serve.sh"
        exit 1
    fi
    sleep 2
done

# Run lychee
echo "Running link checker..."
lychee --config lychee.toml "$URL"