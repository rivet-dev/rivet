#!/usr/bin/env bash
set -euf -o pipefail

nix-shell -p nodejs --command "npx prettier@3.0 --write ."

