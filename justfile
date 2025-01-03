docker_compose := "docker compose -f docker/dev-full/docker-compose.yml"

[group('rust')]
watch:
	bacon

[group('dev')]
dev-compose *ARGS:
	{{docker_compose}} --parallel=1 up -d --build {{ARGS}}

[group('dev')]
dev-up-all:
	{{docker_compose}} --parallel=1 up -d --build

[group('dev')]
dev-up-all-nobuild:
	{{docker_compose}} --parallel=1 up -d

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

