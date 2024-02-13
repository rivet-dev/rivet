#!/usr/bin/env bash
set -euf -o pipefail

FERN_GROUP=runtime ./scripts/fern/gen_inner.sh &
FERN_GROUP=full ./scripts/fern/gen_inner.sh &
wait

