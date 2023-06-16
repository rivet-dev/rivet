#!/usr/bin/env bash
set -euf -o pipefail

# TODO: Ensure in nix-shell

if [ ! "${SKIP_GIT_LFS-0}" == "1" ]; then
	git lfs install
else
	echo "Skipping Git LFS"
fi

# Rebuid bolt
echo
echo '> Building Bolt'
./scripts/bolt/rebuild.sh
