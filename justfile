[group('release')]
release *ARGS:
	./scripts/release/main.ts --phase setup-local {{ ARGS }}

[group('docker')]
docker-build:
	docker build -f engine/docker/universal/Dockerfile --target engine-full -t rivetkit/engine:local --platform linux/x86_64 .

[group('docker')]
docker-build-frontend:
	docker build -f engine/docker/universal/Dockerfile --target engine-full -t rivetkit/engine:local --platform linux/x86_64 --build-arg BUILD_FRONTEND=true .

[group('docker')]
docker-run:
	docker run -p 6420:6420 -e RIVET__AUTH__ADMIN_TOKEN=dev -e RUST_LOG=debug rivetkit/engine:local

