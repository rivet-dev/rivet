#!/usr/bin/env bash
set -euf -o pipefail

(cd sdks && npx -p fern-api fern check --warnings)

