/**
 * Docker manifest helpers used by both preview and release flows.
 *
 * The build (per-arch push) happens in the workflow `docker-images` matrix
 * job. This module composes multi-arch manifests from per-arch images and,
 * on release, retags an existing manifest to the version name.
 *
 * Today we ship `rivetdev/engine:{slim,full}` — add entries to `DOCKER_IMAGES`
 * to tag additional images in lockstep.
 */
import { $ } from "execa";
import { scoped } from "./logger.js";

const log = scoped("docker");

/**
 * Image repos and the set of tag prefixes we publish. `main` marks the image
 * whose *unprefixed* tag (e.g. `rivetdev/engine:{version}`) should also be
 * created on release. Keep `slim` as `main` since it's the default engine.
 */
export interface DockerImage {
	repo: string;
	prefix: string;
	/** When true, also tag unprefixed `{repo}:{version}` on release. */
	main?: boolean;
}

export const DOCKER_IMAGES: readonly DockerImage[] = [
	{ repo: "rivetdev/engine", prefix: "slim", main: true },
	{ repo: "rivetdev/engine", prefix: "full" },
] as const;

/**
 * Create a multi-arch manifest for every image at the given sha from its
 * per-arch source images. Uses `docker buildx imagetools create` which is
 * idempotent — reruns overwrite.
 */
export async function createMultiArchManifests(sha: string): Promise<void> {
	for (const { repo, prefix } of DOCKER_IMAGES) {
		const source = `${repo}:${prefix}-${sha}`;
		const amd = `${repo}:${prefix}-${sha}-amd64`;
		const arm = `${repo}:${prefix}-${sha}-arm64`;
		log.info(`creating multi-arch ${source} from ${amd} + ${arm}`);
		await $({
			stdio: "inherit",
		})`docker buildx imagetools create --tag ${source} ${amd} ${arm}`;
	}
}

/**
 * Retag existing multi-arch manifests from `{prefix}-{fromSha}` to the
 * version-named tags. On release with `latest=true`, also tags the `latest`
 * variants and the unprefixed `{repo}:{version}` / `{repo}:latest` for `main`
 * images.
 */
export async function retagManifestsToVersion(
	fromSha: string,
	version: string,
	latest: boolean,
): Promise<void> {
	for (const img of DOCKER_IMAGES) {
		const source = `${img.repo}:${img.prefix}-${fromSha}`;

		await imagetoolsCreate(img.repo, source, `${img.prefix}-${version}`);
		if (img.main) {
			await imagetoolsCreate(img.repo, source, version);
		}
		if (latest) {
			await imagetoolsCreate(img.repo, source, `${img.prefix}-latest`);
			if (img.main) {
				await imagetoolsCreate(img.repo, source, "latest");
			}
		}
	}
}

async function imagetoolsCreate(
	repo: string,
	source: string,
	targetTag: string,
): Promise<void> {
	const target = `${repo}:${targetTag}`;
	log.info(`retagging ${source} -> ${target}`);
	await $({
		stdio: "inherit",
	})`docker buildx imagetools create --tag ${target} ${source}`;
}
