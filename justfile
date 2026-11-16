[group('release')]
release *ARGS:
	pnpm --filter=publish release {{ ARGS }}

[group('release')]
preview-publish REF:
	gh workflow run .github/workflows/publish.yaml --ref "{{ REF }}"

[group('docker')]
docker-build:
	docker build -f docker/engine/Dockerfile --target engine-full -t rivetdev/engine:local .

[group('docker')]
docker-run:
	docker compose -f self-host/control-plane/docker-compose/docker-compose.yml up -d --wait

[group('docker')]
docker-stop:
	docker compose -f self-host/control-plane/docker-compose/docker-compose.yml stop

[group('skill-evals')]
object-store-regenerate *args:
	python3 vendor/object_store/regenerate.py "$@"

# Regenerate the committed SlateDB source from its pinned release and patch series.
[group('dependencies')]
[positional-arguments]
slatedb-regenerate *args:
	python3 vendor/slatedb/regenerate.py "$@"

# Fail if the committed SlateDB source differs from its patch series.
[group('dependencies')]
[positional-arguments]
slatedb-check *args:
	python3 vendor/slatedb/regenerate.py --check "$@"

# Fail if a BARE schema version that an OSS release shipped has changed.
[group('schemas')]
schemas-check-released:
	python3 scripts/schemas/released_schemas.py

# Re-pin released schema versions after a release. Needs the OSS release tags fetched locally.
[group('schemas')]
schemas-pin-released:
	python3 scripts/schemas/released_schemas.py --update

# Azure fleet deployment recipes live with the deployment configuration.
[group('deploy')]
[positional-arguments]
