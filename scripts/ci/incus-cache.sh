#!/usr/bin/env bash
# Run immediately after checkout, before toolchain/setup actions.
set -euo pipefail
test -n "${CI_CACHE_ROOT:-}"
test -n "${GITHUB_ENV:-}"
scope=$(printf '%s' "$GITHUB_REPOSITORY/$GITHUB_WORKFLOW/$GITHUB_JOB" | sha256sum | cut -c1-20)
cache="$CI_CACHE_ROOT/$scope"
mkdir -p "$cache"
touch "$cache"
attach() {
  local source=$1 destination=$2
  mkdir -p "$source" "$(dirname "$destination")"
  if [ -e "$destination" ] || [ -L "$destination" ]; then
    echo "Refusing to replace existing cache path: $destination" >&2
    exit 1
  fi
  ln -s "$source" "$destination"
}
if [ "${1:-raw}" != docker ]; then
attach "$cache/target" target
mkdir -p "$cache/cargo" "$cache/rustup" "$cache/xdg"
{
  echo "CARGO_HOME=$cache/cargo"
  echo "RUSTUP_HOME=$cache/rustup"
  echo "XDG_CACHE_HOME=$cache/xdg"
} >> "$GITHUB_ENV"
echo "$cache/cargo/bin" >> "$GITHUB_PATH"
fi
# Docker authentication and builder metadata are per job. The remote driver
# owns no daemon: removing this client never deletes the worker's build cache.
docker_config="$RUNNER_TEMP/incus-docker-$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT"
mkdir -p "$docker_config"
export DOCKER_CONFIG="$docker_config"
docker buildx inspect incus >/dev/null 2>&1 ||\
  docker buildx create --name incus --driver remote "$CI_BUILDKIT_ENDPOINT"
{
  echo "DOCKER_CONFIG=$DOCKER_CONFIG"
  echo 'BUILDX_BUILDER=incus'
  echo "CI_CACHE_SCOPE=$CI_SLOT"
  echo "CI_SLOT=$CI_SLOT"
  echo "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"
} >> "$GITHUB_ENV"
