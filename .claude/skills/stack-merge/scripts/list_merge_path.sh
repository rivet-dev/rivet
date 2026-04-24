#!/bin/bash
# Enumerate PRs in the merge path from <target> down to main via baseRefName traversal.
# Output: tab-separated PR# <tab> headBranch <tab> baseBranch, target first, main-adjacent last.
#
# Usage: list_merge_path.sh <target-branch>
set -eu

TARGET="${1:?usage: $0 <target-branch>}"
TRUNK="${2:-main}"

# Trust `gh pr view` baseRefName chain, not `gt ls` visual order.
current="$TARGET"
visited=""

while [ "$current" != "$TRUNK" ]; do
  case " $visited " in
    *" $current "*) echo "ERROR: cycle detected at $current" >&2; exit 1;;
  esac
  visited="$visited $current"

  data=$(gh pr list --head "$current" --state all --json number,headRefName,baseRefName --limit 1 2>/dev/null)
  count=$(echo "$data" | python3 -c "import json,sys;print(len(json.load(sys.stdin)))")
  if [ "$count" -eq 0 ]; then
    echo "ERROR: no PR for branch $current" >&2
    exit 1
  fi

  read -r num head base <<<"$(echo "$data" | python3 -c "import json,sys;d=json.load(sys.stdin)[0];print(d['number'],d['headRefName'],d['baseRefName'])")"
  printf "%s\t%s\t%s\n" "$num" "$head" "$base"

  # graphite-base/<pr#> is Graphite's synthetic parent ref pointing at a commit SHA,
  # not another branch. baseRefName-chained traversal ends here. There may be more
  # PRs further down whose heads are ancestors of this one — Phase 8's
  # "close every open PR reachable from main" step catches those.
  case "$base" in
    graphite-base/*) break;;
  esac

  current="$base"
done
