import { $ } from "execa";

const REPOS = [
	{ name: "rivetkit/engine", prefix: "slim", main: true },
	{ name: "rivetkit/engine", prefix: "full" },
];

export async function tagDocker(opts: {
	version: string;
	commit: string;
	latest: boolean;
}) {
	for (const { name, prefix, main } of REPOS) {
		// Check both architecture images exist using manifest inspect
		console.log(`==> Checking images exist: ${name}:${prefix}-${opts.commit}-{amd64,arm64}`);
		try {
			console.log(`==> Inspecting ${name}:${prefix}-${opts.commit}-amd64`);
			await $`docker manifest inspect ${name}:${prefix}-${opts.commit}-amd64`;
			console.log(`==> Inspecting ${name}:${prefix}-${opts.commit}-arm64`);
			await $`docker manifest inspect ${name}:${prefix}-${opts.commit}-arm64`;
			console.log(`==> Both images exist`);
		} catch (error) {
			console.error(`==> Error inspecting images:`, error);
			throw new Error(
				`Images ${name}:${prefix}-${opts.commit}-{amd64,arm64} do not exist on Docker Hub. Error: ${error}`,
			);
		}

		// Create and push manifest with version
		await createManifest(
			name,
			`${prefix}-${opts.commit}`,
			`${prefix}-${opts.version}`,
		);
		if (main) {
			await createManifest(name, `${prefix}-${opts.commit}`, opts.version);
		}

		// Create and push manifest with latest
		if (opts.latest) {
			await createManifest(name, `${prefix}-${opts.commit}`, `${prefix}-latest`);
			if (main) {
				await createManifest(name, `${prefix}-${opts.commit}`, "latest");
			}
		}
	}
}

async function createManifest(image: string, from: string, to: string) {
	console.log(`==> Creating manifest: ${image}:${to} from ${image}:${from}-{amd64,arm64}`);

	// Use buildx imagetools to create and push multi-arch manifest
	// This works with manifest lists as inputs (unlike docker manifest create)
	await $`docker buildx imagetools create --tag ${image}:${to} ${image}:${from}-amd64 ${image}:${from}-arm64`;
}
