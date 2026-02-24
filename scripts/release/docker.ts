import { $ } from "execa";
import { fetchGitRef, versionOrCommitToRef } from "./utils";

const REPOS = [
	{ name: "rivetdev/engine", prefix: "slim", main: true },
	{ name: "rivetdev/engine", prefix: "full" },
];

export async function tagDocker(opts: {
	version: string;
	commit: string;
	latest: boolean;
	reuseEngineVersion?: string;
}) {
	// Determine how to source images. When reusing a version, use
	// version-tagged manifests directly (works even if that version itself
	// reused engine from an earlier release). When reusing a commit or using
	// the current commit, use per-arch commit-tagged images.
	const useVersionManifest = opts.reuseEngineVersion?.includes(".");

	let sourceTag: string;
	if (opts.reuseEngineVersion) {
		if (useVersionManifest) {
			sourceTag = opts.reuseEngineVersion;
			console.log(`==> Reusing version-tagged manifests from ${sourceTag}`);
		} else {
			console.log(`==> Reusing artifacts from commit ${opts.reuseEngineVersion}`);
			const ref = versionOrCommitToRef(opts.reuseEngineVersion);
			await fetchGitRef(ref);
			const result = await $`git rev-parse ${ref}`;
			sourceTag = result.stdout.trim().slice(0, 7);
			console.log(`==> Source commit: ${sourceTag}`);
		}
	} else {
		sourceTag = opts.commit;
	}

	for (const { name, prefix, main } of REPOS) {
		if (useVersionManifest) {
			// Verify version-tagged manifest exists.
			const manifestTag = `${prefix}-${sourceTag}`;
			console.log(`==> Checking manifest exists: ${name}:${manifestTag}`);
			try {
				await $({ stdio: "inherit" })`docker manifest inspect ${name}:${manifestTag}`;
				console.log(`==> Manifest exists`);
			} catch (error) {
				throw new Error(
					`Manifest ${name}:${manifestTag} does not exist on Docker Hub. Error: ${error}`,
				);
			}

			// Create new manifests by retagging the existing version manifest.
			await retagManifest(name, `${prefix}-${sourceTag}`, `${prefix}-${opts.version}`);
			if (main) {
				await retagManifest(name, `${prefix}-${sourceTag}`, opts.version);
			}

			if (opts.latest) {
				await retagManifest(name, `${prefix}-${sourceTag}`, `${prefix}-latest`);
				if (main) {
					await retagManifest(name, `${prefix}-${sourceTag}`, "latest");
				}
			}
		} else {
			// Check both per-arch images exist.
			console.log(`==> Checking images exist: ${name}:${prefix}-${sourceTag}-{amd64,arm64}`);
			try {
				console.log(`==> Inspecting ${name}:${prefix}-${sourceTag}-amd64`);
				await $({ stdio: "inherit" })`docker manifest inspect ${name}:${prefix}-${sourceTag}-amd64`;
				console.log(`==> Inspecting ${name}:${prefix}-${sourceTag}-arm64`);
				await $({ stdio: "inherit" })`docker manifest inspect ${name}:${prefix}-${sourceTag}-arm64`;
				console.log(`==> Both images exist`);
			} catch (error) {
				console.error(`==> Error inspecting images:`, error);
				throw new Error(
					`Images ${name}:${prefix}-${sourceTag}-{amd64,arm64} do not exist on Docker Hub. Error: ${error}`,
				);
			}

			// Create multi-arch manifests from per-arch images.
			await createManifestFromArch(name, `${prefix}-${sourceTag}`, `${prefix}-${opts.version}`);
			if (main) {
				await createManifestFromArch(name, `${prefix}-${sourceTag}`, opts.version);
			}

			if (opts.latest) {
				await createManifestFromArch(name, `${prefix}-${sourceTag}`, `${prefix}-latest`);
				if (main) {
					await createManifestFromArch(name, `${prefix}-${sourceTag}`, "latest");
				}
			}
		}
	}
}

/** Create a new multi-arch manifest by retagging an existing manifest. */
async function retagManifest(image: string, from: string, to: string) {
	console.log(`==> Retagging manifest: ${image}:${to} from ${image}:${from}`);
	await $({ stdio: "inherit" })`docker buildx imagetools create --tag ${image}:${to} ${image}:${from}`;
}

/** Create a new multi-arch manifest from per-arch images. */
async function createManifestFromArch(image: string, from: string, to: string) {
	console.log(`==> Creating manifest: ${image}:${to} from ${image}:${from}-{amd64,arm64}`);
	await $({ stdio: "inherit" })`docker buildx imagetools create --tag ${image}:${to} ${image}:${from}-amd64 ${image}:${from}-arm64`;
}
