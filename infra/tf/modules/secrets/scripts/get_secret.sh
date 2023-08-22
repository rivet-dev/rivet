#!/usr/bin/env bash
set -euf

export BOLT_HEADLESS=1

KEY="$1"
OPTIONAL="$2"

if [ "$OPTIONAL" == "true" ]; then
	bolt secret get --format=json --optional "$KEY"
else
	bolt secret get --format=json "$KEY"
fi

