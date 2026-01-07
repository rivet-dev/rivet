#!/bin/sh
# Fetch Git LFS files from pointer files
# When the build platform does not natively support Git LFS, we manually resolve
# the OID pointer files to their actual content

set -e

REPO_URL="${1:-https://github.com/rivet-gg/rivet.git}"

git init
git remote add origin "$REPO_URL"
git lfs install

find . \( -name "*.gif" -o -name "*.png" -o -name "*.jpg" -o -name "*.jpeg" -o -name "*.webp" -o -name "*.mp4" -o -name "*.webm" \) -print0 | \
xargs -0 -P 8 -I {} sh -c '
  if head -c 100 "{}" 2>/dev/null | grep -q "^version https://git-lfs"; then
    echo "Fetching LFS: {}"
    git lfs smudge < "{}" > "{}.tmp" && mv "{}.tmp" "{}"
  fi
'
