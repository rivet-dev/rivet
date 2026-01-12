import { $ } from "execa";
import { fetchGitRef, versionOrCommitToRef } from "./utils";

const REPOS = [
	{ name: "rivetkit/engine", prefix: "slim", main: true },
	{ name: "rivetkit/engine", prefix: "full" },
];

export async function tagDocker(opts: {
	version: string;
	commit: string;
	latest: boolean;
	reuseEngineVersion?: string;
}) {
	// Determine which commit to use for source images
	let sourceCommit = opts.commit;
	if (opts.reuseEngineVersion) {
		console.log(`==> Reusing artifacts from ${opts.reuseEngineVersion}`);
		const ref = versionOrCommitToRef(opts.reuseEngineVersion);
		await fetchGitRef(ref);
		const result = await $`git rev-parse ${ref}`;
		sourceCommit = result.stdout.trim().slice(0, 7);
		console.log(`==> Source commit: ${sourceCommit}`);
	}

	for (const { name, prefix, main } of REPOS) {
		// Check both architecture images exist using manifest inspect
		console.log(`==> Checking images exist: ${name}:${prefix}-${sourceCommit}-{amd64,arm64}`);
		try {
			console.log(`==> Inspecting ${name}:${prefix}-${sourceCommit}-amd64`);
			await $({ stdio: "inherit" })`docker manifest inspect ${name}:${prefix}-${sourceCommit}-amd64`;
			console.log(`==> Inspecting ${name}:${prefix}-${sourceCommit}-arm64`);
			await $({ stdio: "inherit" })`docker manifest inspect ${name}:${prefix}-${sourceCommit}-arm64`;
			console.log(`==> Both images exist`);
		} catch (error) {
			console.error(`==> Error inspecting images:`, error);
			throw new Error(
				`Images ${name}:${prefix}-${sourceCommit}-{amd64,arm64} do not exist on Docker Hub. Error: ${error}`,
			);
		}

		// Create and push manifest with version
		await createManifest(
			name,
			`${prefix}-${sourceCommit}`,
			`${prefix}-${opts.version}`,
		);
		if (main) {
			await createManifest(name, `${prefix}-${sourceCommit}`, opts.version);
		}

		// Create and push manifest with latest
		if (opts.latest) {
			await createManifest(name, `${prefix}-${sourceCommit}`, `${prefix}-latest`);
			if (main) {
				await createManifest(name, `${prefix}-${sourceCommit}`, "latest");
			}
		}
	}
}

async function createManifest(image: string, from: string, to: string) {
	console.log(`==> Creating manifest: ${image}:${to} from ${image}:${from}-{amd64,arm64}`);

	// Use buildx imagetools to create and push multi-arch manifest
	// This works with manifest lists as inputs (unlike docker manifest create)
	await $({ stdio: "inherit" })`docker buildx imagetools create --tag ${image}:${to} ${image}:${from}-amd64 ${image}:${from}-arm64`;
}
