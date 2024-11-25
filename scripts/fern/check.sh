#!/usr/bin/env bash
set -euf -o pipefail

(cd sdks/api && npx -p fern-api@0.44.11 fern check --warnings)

