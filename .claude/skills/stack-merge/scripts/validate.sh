#!/bin/bash
# Phase 5 read-only validation. Checks preconditions for a safe FF push of main to <target>.
# Exits 0 if safe, 1 if any hard check fails.
#
# Usage: validate.sh <target-branch> [<trunk-branch>]
set -eu

TARGET="${1:?usage: $0 <target-branch> [trunk]}"
TRUNK="${2:-main}"
REPO="rivet-dev/rivet"
HARD_FAIL=0

echo "=== Fetching latest refs ==="
git fetch origin "$TRUNK" "$TARGET" 2>&1 | tail -3 || true
echo

# 1. FF safety
echo "=== 1. FF safety: is $TRUNK an ancestor of $TARGET? ==="
if git merge-base --is-ancestor "origin/$TRUNK" "origin/$TARGET"; then
  echo "  PASS"
else
  echo "  FAIL - $TRUNK has diverged from $TARGET"
  HARD_FAIL=1
fi
echo

# 2. Zero commits would be lost from main
echo "=== 2. Commits on $TRUNK not in $TARGET (must be 0) ==="
lost=$(git rev-list --count "origin/$TARGET..origin/$TRUNK")
if [ "$lost" -eq 0 ]; then
  echo "  PASS (0)"
else
  echo "  FAIL - $lost commits on $TRUNK would be lost:"
  git log --oneline "origin/$TARGET..origin/$TRUNK" | head -20 | sed 's/^/    /'
  HARD_FAIL=1
fi
echo

# 3. Admin / bypass permissions
echo "=== 3. Admin or bypass permissions on $TRUNK ==="
me=$(gh api user --jq .login 2>/dev/null)
perm=$(gh api "repos/$REPO/collaborators/$me/permission" --jq .permission 2>/dev/null || echo unknown)
bypass=$(gh api "repos/$REPO/branches/$TRUNK/protection" --jq '.required_pull_request_reviews.bypass_pull_request_allowances.users[].login' 2>/dev/null | tr '\n' ' ')
if [ "$perm" = "admin" ]; then
  echo "  PASS ($me has repo admin)"
elif echo " $bypass " | grep -q " $me "; then
  echo "  PASS ($me is in bypass list)"
else
  echo "  FAIL - $me has perm=$perm, bypass list='$bypass'"
  HARD_FAIL=1
fi
echo

# 4. Commits landing
echo "=== 4. Commits landing on $TRUNK ==="
landing=$(git rev-list --count "origin/$TRUNK..origin/$TARGET")
echo "  $landing commits will land"
echo

# 5. Commit hygiene (informational, non-blocking)
echo "=== 5. Commit hygiene (informational) ==="
"$(dirname "$0")/detect_ralph.sh" "$TARGET" "$TRUNK" || true
echo

# 6. Conflict preview via scratch merge
echo "=== 6. Conflict preview (main into target) ==="
tmp=$(mktemp -d)
if git clone --no-checkout --quiet --local "$PWD" "$tmp/probe" 2>/dev/null && \
   (cd "$tmp/probe" && git remote set-url origin "https://github.com/$REPO.git" && \
     git fetch origin "$TRUNK" "$TARGET" --quiet 2>/dev/null && \
     git checkout -b probe-merge "origin/$TARGET" --quiet 2>/dev/null && \
     git merge "origin/$TRUNK" --no-commit --no-ff 2>&1 | tail -20 | sed 's/^/  /'); then
  :
fi
rm -rf "$tmp"
echo

# Summary
echo "============================================================"
if [ "$HARD_FAIL" -eq 0 ]; then
  echo "VALIDATION PASSED - safe to proceed to confirmation gate"
  exit 0
else
  echo "VALIDATION FAILED - resolve hard failures before pushing"
  exit 1
fi
