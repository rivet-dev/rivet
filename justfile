# `--parallel 1` because apt has issues when pulling too many containers in parallel
docker_compose := "docker compose -f docker/dev-full/docker-compose.yml --progress=plain --parallel 1"

[group('rust')]
watch:
	bacon

[group('run')]
[no-cd]
cli *ARGS:
	cargo run -p rivet-cli -- {{ARGS}}

[group('dev')]
dev-compose *ARGS:
	{{docker_compose}} {{ARGS}}

[group('dev')]
dev-up-all:
	{{docker_compose}} up -d --build

[group('dev')]
dev-up-all-nobuild:
	{{docker_compose}} up -d

[group('dev')]
dev-up CONTAINER:
	{{docker_compose}} up -d --build {{CONTAINER}}

[group('dev')]
dev-up-nobuild CONTAINER:
	{{docker_compose}} up -d {{CONTAINER}}

[group('dev')]
dev-logs CONTAINER:
	{{docker_compose}} logs -f {{CONTAINER}}

[group('dev')]
dev-logs-client:
	{{docker_compose}} exec rivet-client sh -c 'tail -f -n 100 /var/lib/rivet-client/logs/*'

[group('dev')]
dev-logs-runner:
	{{docker_compose}} exec rivet-client sh -c 'tail -f -n 100 /var/lib/rivet-client/runner/logs/*'

[group('dev')]
dev-logs-server:
	{{docker_compose}} logs -f rivet-server

[group('dev')]
dev-exec CONTAINER:
	{{docker_compose}} exec -it {{CONTAINER}} /bin/bash

[group('dev')]
dev-cmd *ARGS:
	{{docker_compose}} exec -it rivet-server rivet-server {{ARGS}}

[group('dev')]
dev-down:
	{{docker_compose}} down

[group('dev')]
dev-nuke:
	{{docker_compose}} down -v -t 0

[group('fern')]
fern-check:
	./scripts/fern/check.sh

[group('fern')]
fern-gen:
	./scripts/fern/gen.sh

[group('actor')]
actor-compile-bridge:
	./scripts/sdk_actor/compile_bridge.ts

[group('actor')]
actor-check: actor-compile-bridge
	cd sdks/actor && deno check --all client/**/*.ts runtime/**/*.ts client/**/*.ts manager/**/*.ts && biome check --write

[group('system')]
system-test REGION="":
	cd examples/system-test && REGION={{REGION}} rivet deno --populate-env run -A ws_test.ts

[group('system')]
system-test-login:
	cd examples/system-test && rivet login

alias gcs := graphite-create-submit

[group('graphite')]
graphite-create-submit MESSAGE REVIEWER:
	gt create --all --message '{{MESSAGE}}'
	gt submit --no-edit --publish --reviewers '{{REVIEWER}}'

alias gm := graphite-modify

[group('graphite')]
graphite-modify:
	gt modify -a
	gt submit

[group('github')]
release-latest VERSION:
	gh workflow run .github/workflows/release.yaml -f version={{ VERSION }} -f latest=true

[group('github')]
release-nolatest VERSION:
	gh workflow run .github/workflows/release.yaml -f version={{ VERSION }} -f latest=false

