#!/bin/sh
set -euf

(cd infra/default-builds && ./build_all.sh)
kubectl delete jobs.batch -n rivet-service rivet-build-default-create
bolt up build-default-create

