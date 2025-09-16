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

