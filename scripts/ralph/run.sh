#!/bin/bash
# ralph run script
# Usage: ./run.sh <iterations>

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROMPT_FILE="$SCRIPT_DIR/prompt.txt"
OUTPUT_FILE="$SCRIPT_DIR/output.txt"

if [ -z "$1" ]; then
  echo "Usage: $0 <iterations>"
  exit 1
fi

if [ ! -f "$PROMPT_FILE" ]; then
  echo "Error: prompt.txt not found at $PROMPT_FILE"
  exit 1
fi

PROMPT=$(cat "$PROMPT_FILE")

for ((i=1; i<=$1; i++)); do
  echo "=== Starting iteration $i ==="

  claude -p "$PROMPT" --output-format stream-json --verbose --dangerously-skip-permissions \
    | tee "$OUTPUT_FILE" \
    | jq -r 'select(.type == "assistant") | .message.content[]? | select(.type == "text") | .text // empty' 2>/dev/null || true

  if grep -q "<promise>COMPLETE</promise>" "$OUTPUT_FILE"; then
    echo "PRD complete, exiting."
    exit 0
  fi
done

