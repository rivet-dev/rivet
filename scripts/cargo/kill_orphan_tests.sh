#!/bin/bash

# Kill orphaned Rust test processes

echo "Looking for orphaned test processes..."

# Find all test processes running from target directory
PIDS=$(ps aux | grep -E "target/" | grep -v grep | awk '{print $2}')

if [ -z "$PIDS" ]; then
    echo "No orphaned test processes found."
    exit 0
fi

echo "Found orphaned test processes:"
ps aux | grep -E "target/.*test" | grep -v grep

echo -e "\nKilling processes..."
for PID in $PIDS; do
    echo "Killing PID: $PID"
    kill -9 $PID 2>/dev/null || echo "Failed to kill PID: $PID"
done

echo -e "\nDone."
