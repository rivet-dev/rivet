#!/bin/sh

(cd ../.. && docker build -f cli.Dockerfile -t opengb .) && docker run --privileged -it --add-host=host.docker.internal:host-gateway -e DATABASE_URL=postgres://postgres:postgres@host.docker.internal:5432/postgres?sslmode=disable -e VERBOSE=1 -v ./:/backend -w /backend opengb dev
# (cd ../.. && deno task cli:install) && opengb clean && opengb dev

