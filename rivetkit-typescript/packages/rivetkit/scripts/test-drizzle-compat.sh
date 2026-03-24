#!/usr/bin/env bash
#
# Tests rivetkit's drizzle integration against multiple drizzle-orm versions.
#
# Usage:
#   ./scripts/test-drizzle-compat.sh                   # test all versions
#   ./scripts/test-drizzle-compat.sh 0.44.2 0.45.1     # test specific versions
#
# Run from rivetkit-typescript/packages/rivetkit/.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT_DIR="$(cd "$PKG_DIR/../../.." && pwd)"

# Default versions to test. Uses the latest patch of each minor release.
# Add new minor versions here as drizzle releases them.
DEFAULT_VERSIONS=("0.44" "0.45")

if [[ $# -gt 0 ]]; then
	VERSIONS=("$@")
else
	VERSIONS=("${DEFAULT_VERSIONS[@]}")
fi

cd "$ROOT_DIR"

# Back up files that pnpm add will modify.
cp "$PKG_DIR/package.json" "$PKG_DIR/package.json.drizzle-compat-bak"
cp "$ROOT_DIR/pnpm-lock.yaml" "$ROOT_DIR/pnpm-lock.yaml.drizzle-compat-bak"

cleanup() {
	echo ""
	echo "Restoring original package.json and lockfile..."
	mv "$PKG_DIR/package.json.drizzle-compat-bak" "$PKG_DIR/package.json"
	mv "$ROOT_DIR/pnpm-lock.yaml.drizzle-compat-bak" "$ROOT_DIR/pnpm-lock.yaml"
	cd "$ROOT_DIR" && pnpm install --frozen-lockfile 2>/dev/null || pnpm install
}
trap cleanup EXIT

declare -A RESULTS

for version in "${VERSIONS[@]}"; do
	echo ""
	echo "=========================================="
	echo "  Testing drizzle-orm@$version"
	echo "=========================================="

	# Install the target version into the rivetkit package.
	pnpm --filter rivetkit add -D "drizzle-orm@$version" 2>&1 | tail -3

	# Run only the drizzle variant of the DB tests.
	TEST_LOG="/tmp/drizzle-compat-${version}.log"
	if cd "$PKG_DIR" && pnpm test driver-file-system -t "Actor Database \(drizzle\)" > "$TEST_LOG" 2>&1; then
		RESULTS["$version"]="PASS"
		echo "  -> PASS"
	else
		RESULTS["$version"]="FAIL"
		echo "  -> FAIL (see $TEST_LOG)"
	fi
	cd "$ROOT_DIR"
done

echo ""
echo "=========================================="
echo "  Drizzle Compatibility Results"
echo "=========================================="
for version in "${VERSIONS[@]}"; do
	printf "  drizzle-orm@%-10s %s\n" "$version" "${RESULTS[$version]}"
done
