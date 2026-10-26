#!/usr/bin/env bash
set -euo pipefail

artifact_dir="${1:?engine artifact directory is required}"
package_dir="rivetkit-typescript/packages/engine-cli/npm"

platforms=(
    'rivet-engine-x86_64-unknown-linux-musl:linux-x64-musl'
    'rivet-engine-aarch64-unknown-linux-musl:linux-arm64-musl'
    'rivet-engine-x86_64-apple-darwin:darwin-x64'
    'rivet-engine-aarch64-apple-darwin:darwin-arm64'
)

# Validate the whole set before replacing any binary. A missing build must
# never let an older binary already in a package reach the registry.
for entry in "${platforms[@]}"; do
    artifact="${entry%:*}"
    platform="${entry#*:}"
    if [ ! -f "$artifact_dir/$artifact" ] || [ ! -s "$artifact_dir/$artifact" ]; then
        echo "Missing fresh engine artifact: $artifact_dir/$artifact" >&2
        exit 1
    fi
    if [ ! -f "$package_dir/$platform/package.json" ]; then
        echo "Missing engine package: $package_dir/$platform" >&2
        exit 1
    fi
done

for entry in "${platforms[@]}"; do
    artifact="${entry%:*}"
    platform="${entry#*:}"
    binary="rivet-engine"
    cp -- "$artifact_dir/$artifact" "$package_dir/$platform/$binary"
    chmod +x "$package_dir/$platform/$binary"
    echo "Placed $artifact -> npm/$platform/$binary"
done
