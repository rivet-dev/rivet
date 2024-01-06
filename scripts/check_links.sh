#!/bin/sh
set -euf

nix-shell -p lychee --run "lychee ."

