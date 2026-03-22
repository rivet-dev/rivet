/**
 * Docker helpers — build, login, tag, and push Docker images.
 * Uses Bun's built-in shell operator for subprocess execution.
 */

import { $ } from "bun";
import { colors } from "../utils/output.ts";

export interface BuildResult {
	imageId: string;
}

/**
 * Run `docker build` and return the local image ID.
 * We use `--iidfile` to capture the image digest reliably.
 */
export async function dockerBuild(opts: {
	context: string;
	dockerfile?: string;
	platform?: string;
}): Promise<BuildResult> {
	const iidFile = `/tmp/rivet-cloud-cli-iid-${Date.now()}`;

	const dockerfileArgs = opts.dockerfile ? ["-f", opts.dockerfile] : [];
	const platformArgs = opts.platform ? ["--platform", opts.platform] : [];

	try {
		await $`docker build ${dockerfileArgs} ${platformArgs} --iidfile ${iidFile} ${opts.context}`.quiet();
	} catch (err) {
		throw new Error(
			`Docker build failed. Make sure Docker is running and a Dockerfile exists in ${opts.context}.\n${String(err)}`,
		);
	}

	const imageId = (await Bun.file(iidFile).text()).trim();
	return { imageId };
}

/**
 * Login to a Docker registry using `docker login`.
 */
export async function dockerLogin(opts: {
	registryUrl: string;
	username: string;
	password: string;
}): Promise<void> {
	try {
		const proc = Bun.spawn(
			["docker", "login", opts.registryUrl, "--username", opts.username, "--password-stdin"],
			{
				stdin: new TextEncoder().encode(opts.password),
				stdout: "ignore",
				stderr: "ignore",
			},
		);
		const exitCode = await proc.exited;
		if (exitCode !== 0) {
			throw new Error(`Docker login exited with code ${exitCode}`);
		}
	} catch (err) {
		throw new Error(
			`Docker login failed for registry ${opts.registryUrl}.\n${String(err)}`,
		);
	}
}

/**
 * Tag a local image and push it to a remote registry.
 * Returns the full remote image reference (e.g. registry.rivet.dev/org/proj/repo:tag).
 */
export async function dockerTagAndPush(opts: {
	localImageId: string;
	remoteRef: string;
}): Promise<void> {
	try {
		await $`docker tag ${opts.localImageId} ${opts.remoteRef}`.quiet();
		await $`docker push ${opts.remoteRef}`.quiet();
	} catch (err) {
		throw new Error(`Docker tag/push failed for ${opts.remoteRef}.\n${String(err)}`);
	}
}

/**
 * Check whether the `docker` binary is reachable.
 */
export async function assertDockerAvailable(): Promise<void> {
	try {
		await $`docker info`.quiet();
	} catch {
		console.error(
			colors.error(
				"Docker is not running or not installed. Please start Docker and try again.",
			),
		);
		process.exit(1);
	}
}
