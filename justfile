[group('release')]
release *ARGS:
	pnpm --filter=publish release {{ ARGS }}

[group('release')]
preview-publish REF:
	gh workflow run .github/workflows/publish.yaml --ref "{{ REF }}"

[group('docker')]
docker-build:
	docker build -f docker/engine/universal/Dockerfile --target engine-full -t rivetdev/engine:local --platform linux/x86_64 .

[group('docker')]
docker-run:
	docker run -p 6420:6420 -e RIVET__AUTH__ADMIN_TOKEN=dev -e RUST_LOG=debug rivetdev/engine:local

[group('docker')]
docker-stop:
	docker run -p 6420:6420 -e RIVET__AUTH__ADMIN_TOKEN=dev -e RUST_LOG=debug rivetdev/engine:local

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

# Azure fleet deployment recipes live with the deployment configuration.
[group('deploy')]
[positional-arguments]
