#!/bin/sh
# See docs-internal/engine/RELEASING.md
set -e

if [ "$1" != "--force" ]; then
	git fetch origin main
	if [ "$(git branch --show-current)" != "main" ] || [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
		echo "Error: Must be on main branch and up to date with remote (use --force to override)"
		exit 1
	fi
fi

git push --force origin HEAD:prod
