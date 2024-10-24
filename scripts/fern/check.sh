#!/usr/bin/env bash
set -euf -o pipefail

(cd sdks && fern check)

