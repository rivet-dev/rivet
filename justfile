[group('release')]
release *ARGS:
	pnpm --filter=publish release {{ ARGS }}

[group('release')]
preview-publish REF:
	gh workflow run .github/workflows/publish.yaml --ref "{{ REF }}"

# Point rivet at the sibling ../agent-os checkout for local hacking on agent-os
# (npm `link:`, cargo `path`). The local dev loop: edit agent-os, rebuild here.
[group('agent-os')]
agent-os-local:
	node scripts/agent-os-dep.mjs local

# Point rivet at PUBLISHED agent-os versions (CI/release default).
[group('agent-os')]
agent-os-pinned:
	node scripts/agent-os-dep.mjs pinned

# Bump the pinned @rivet-dev/agent-os-* npm version (after an agent-os preview publish).
[group('agent-os')]
agent-os-set-version VERSION:
	node scripts/agent-os-dep.mjs set-version "{{ VERSION }}"

# Show the current agent-os dependency mode + pinned versions.
[group('agent-os')]
agent-os-status:
	node scripts/agent-os-dep.mjs status

[group('docker')]
docker-build:
	docker build -f engine/docker/universal/Dockerfile --target engine-full -t rivetdev/engine:local --platform linux/x86_64 .

[group('docker')]
docker-build-frontend:
	docker build -f engine/docker/universal/Dockerfile --target engine-full -t rivetdev/engine:local --platform linux/x86_64 --build-arg BUILD_FRONTEND=true .

[group('docker')]
docker-run:
	docker run -p 6420:6420 -e RIVET__AUTH__ADMIN_TOKEN=dev -e RUST_LOG=debug rivetdev/engine:local

[group('docker')]
docker-stop:
	docker run -p 6420:6420 -e RIVET__AUTH__ADMIN_TOKEN=dev -e RUST_LOG=debug rivetdev/engine:local

[group('skill-evals')]
skill-eval name *args:
	cd scripts/skill-evals && npx tsx src/index.ts --eval {{name}} {{args}}

[group('skill-evals')]
skill-eval-clean:
	rm -rf scripts/skill-evals/results
