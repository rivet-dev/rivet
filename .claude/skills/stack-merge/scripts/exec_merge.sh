#!/bin/bash
# Phase 7 — executes the two writes that land the stack.
# Requires --confirm to actually run; without it, dry-runs and prints commands.
#
# Usage: exec_merge.sh <target-branch> [--confirm] [<trunk-branch>]
set -eu

TARGET="${1:?usage: $0 <target-branch> [--confirm] [trunk]}"
shift || true
CONFIRM=0
TRUNK="main"
for arg in "$@"; do
  case "$arg" in
    --confirm) CONFIRM=1 ;;
    *) TRUNK="$arg" ;;
  esac
done

# Enumerate merge-path branches via list_merge_path.sh
branches=()
while IFS=$'\t' read -r _ head _; do
  branches+=("$head")
done < <("$(dirname "$0")/list_merge_path.sh" "$TARGET" "$TRUNK")

echo "=== Merge path ($((${#branches[@]})) branches) ==="
printf '  %s\n' "${branches[@]}"
echo

# Re-verify FF safety right before the push
if ! git merge-base --is-ancestor "origin/$TRUNK" "origin/$TARGET"; then
  echo "ERROR: $TRUNK is not an ancestor of $TARGET - aborting"
  exit 1
fi
if [ "$(git rev-list --count "origin/$TARGET..origin/$TRUNK")" -ne 0 ]; then
  echo "ERROR: commits on $TRUNK not in $TARGET - aborting"
  exit 1
fi

echo "=== Push 1: batch force-push merge-path branches ==="
cmd=(git push origin --force-with-lease "${branches[@]}")
printf '  '; printf '%q ' "${cmd[@]}"; echo

echo "=== Push 2: FF push $TRUNK to target tip ==="
echo "  git push origin origin/$TARGET:$TRUNK"
echo

if [ "$CONFIRM" -ne 1 ]; then
  echo "DRY RUN - re-run with --confirm to execute."
  exit 0
fi

echo "=== Executing Push 1 ==="
"${cmd[@]}"
echo
echo "=== Executing Push 2 ==="
git push origin "origin/$TARGET:$TRUNK"
echo
echo "=== Done. Run Phase 8 verification next. ==="
