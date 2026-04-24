#!/bin/bash
# Flag Ralph-style commits in the merge path (informational).
# Pattern: "<type>(<scope>)?!: [US-NNN] - desc"  or "<type>: US-NNN - desc"
#
# Usage: detect_ralph.sh <target-branch> [<trunk-branch>]
set -eu

TARGET="${1:?usage: $0 <target-branch> [trunk]}"
TRUNK="${2:-main}"

RALPH_RE='^[a-z]+(\([a-z0-9_/-]+\))?!?:[[:space:]]*\[?US-[0-9]+\]?[[:space:]]*-[[:space:]]'

total=$(git log --format="%s" "origin/$TRUNK..origin/$TARGET" 2>/dev/null | wc -l)
ralph=$(git log --format="%s" "origin/$TRUNK..origin/$TARGET" 2>/dev/null | grep -cE "$RALPH_RE" || true)

echo "  Ralph-style commits: $ralph / $total commits in $TRUNK..$TARGET"

if [ "$ralph" -gt 0 ]; then
  echo "  (informational — consider squashing branches with Ralph-style commits)"
  echo
  echo "  First few matches:"
  git log --format="%h %s" "origin/$TRUNK..origin/$TARGET" | grep -E " [a-z]+(\([a-z0-9_/-]+\))?!?:[[:space:]]*\[?US-[0-9]+\]?[[:space:]]*-[[:space:]]" | head -5 | sed 's/^/    /'
fi
