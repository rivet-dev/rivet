/**
 * Docker helpers — build, login, tag, and push Docker images.
 * Uses Bun's built-in shell operator for subprocess execution.
 */

import type { Writable } from "node:stream";
import { $ } from "bun";
import { colors, error, fatal } from "../utils/output.ts";

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
	stream?: Writable;
}): Promise<BuildResult> {
	const iidFile = `/tmp/rivet-cloud-cli-iid-${Date.now()}`;

	const dockerfileArgs = opts.dockerfile ? ["-f", opts.dockerfile] : [];
	const platformArgs = opts.platform ? ["--platform", opts.platform] : [];

	const proc = Bun.spawn([
		"docker",
		"build",
		...dockerfileArgs,
		...platformArgs,
		"--iidfile",
		iidFile,
		opts.context,
	]);

	if (opts.stream) {
		pipeToWritable(proc.stdout, opts.stream);
		pipeToWritable(proc.stderr, opts.stream);
	}

	const exitCode = await proc.exited;
	if (exitCode !== 0) {
		throw error(
			`Docker build failed. Make sure Docker is running and a Dockerfile exists in ${opts.context}.`,
			`exit code: ${exitCode}`,
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
	stream?: Writable;
}): Promise<void> {
	const command = [
		"docker",
		"login",
		opts.registryUrl,
		"--username",
		opts.username,
		"--password-stdin",
	];

	opts?.stream?.write(`$ ${command.join(" ")}\n`);

	const proc = Bun.spawn(
		command,
		{
			stdin: new TextEncoder().encode(opts.password),
			stderr: "pipe",
			stdout: "pipe",
		},
	);

	if (opts.stream) {
		pipeToWritable(proc.stdout, opts.stream);
		pipeToWritable(proc.stderr, opts.stream);
	}

	const exitCode = await proc.exited;
	if (exitCode !== 0) {
		throw error(
			`Docker login failed for registry ${opts.registryUrl}.`,
			`exit code: ${exitCode}`,
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
	stream?: Writable;
}): Promise<void> {
	const tagProc = Bun.spawn([
		"docker",
		"tag",
		opts.localImageId,
		opts.remoteRef,
	]);

	if (opts.stream) {
		pipeToWritable(tagProc.stdout, opts.stream);
		pipeToWritable(tagProc.stderr, opts.stream);
	}

	const tagExitCode = await tagProc.exited;
	if (tagExitCode !== 0) {
		throw error(
			`Docker tag failed for ${opts.remoteRef}.`,
			`exit code: ${tagExitCode}`,
		);
	}

	const pushProc = Bun.spawn(["docker", "push", opts.remoteRef]);

	if (opts.stream) {
		pipeToWritable(pushProc.stdout, opts.stream);
		pipeToWritable(pushProc.stderr, opts.stream);
	}

	const pushExitCode = await pushProc.exited;
	if (pushExitCode !== 0) {
		throw error(
			`Docker push failed for ${opts.remoteRef}.`,
			`exit code: ${pushExitCode}`,
		);
	}
}

/**
 * Check whether the `docker` binary is reachable.
 */
export async function assertDockerAvailable(): Promise<void> {
	try {
		await $`docker info`.quiet();
	} catch {
		fatal(
			"Docker is not available. Please install Docker and make sure it's running.",
		);
	}
}

function pipeToWritable(
	source: ReadableStream<Uint8Array> | undefined,
	dest: Writable,
): void {
	if (!source) return;
	source.pipeTo(
		new WritableStream({
			write(chunk) {
				dest.write(chunk);
			},
		}),
	);
}
