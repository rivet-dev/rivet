[group('github')]
release-latest VERSION:
	./scripts/release/main.ts --setupLocal --version {{ VERSION }}
	gh workflow run .github/workflows/release.yaml -f version={{ VERSION }} -f latest=true --ref  $(git branch --show-current)
	echo 'Once workflow is complete, manually merge Release Please'

[group('github')]
release-nolatest VERSION:
	./scripts/release/main.ts --setupLocal --version {{ VERSION }} --no-latest
	gh workflow run .github/workflows/release.yaml -f version={{ VERSION }} -f latest=false --ref $(git branch --show-current)
	echo 'Once workflow is complete, manually merge Release Please'

[group('docker')]
docker-build:
	docker build -f docker/universal/Dockerfile --target engine-full -t rivetkit/engine:local --platform linux/x86_64 .

[group('docker')]
docker-build-frontend:
	docker build -f docker/universal/Dockerfile --target engine-full -t rivetkit/engine:local --platform linux/x86_64 --build-arg BUILD_FRONTEND=true .

[group('docker')]
docker-run:
	docker run -p 6420:6420 -e RIVET__AUTH__ADMIN_TOKEN=dev -e RUST_LOG=debug rivetkit/engine:local

