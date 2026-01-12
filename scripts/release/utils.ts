import * as fs from "node:fs/promises";
import { $ } from "execa";

export function assert(condition: any, message?: string): asserts condition {
	if (!condition) {
		throw new Error(message || "Assertion failed");
	}
}

/**
 * Converts a version string or commit hash to a git ref.
 * If the input contains a dot, it's treated as a version (e.g., "2.0.33" -> "v2.0.33").
 * Otherwise, it's treated as a git revision and returned as-is (e.g., "bb7f292").
 */
export function versionOrCommitToRef(versionOrCommit: string): string {
	if (versionOrCommit.includes(".")) {
		assert(
			!versionOrCommit.startsWith("v"),
			`Version should not start with "v" (got "${versionOrCommit}", use "${versionOrCommit.slice(1)}" instead)`,
		);
		return `v${versionOrCommit}`;
	}
	return versionOrCommit;
}

interface ReleasesS3Config {
	awsEnv: Record<string, string>;
	endpointUrl: string;
}

let cachedConfig: ReleasesS3Config | null = null;

async function getReleasesS3Config(): Promise<ReleasesS3Config> {
	if (cachedConfig) {
		return cachedConfig;
	}

	let awsAccessKeyId = process.env.R2_RELEASES_ACCESS_KEY_ID;
	if (!awsAccessKeyId) {
		const result =
			await $`op read "op://Engineering/rivet-releases R2 Upload/username"`;
		awsAccessKeyId = result.stdout.trim();
	}
	let awsSecretAccessKey = process.env.R2_RELEASES_SECRET_ACCESS_KEY;
	if (!awsSecretAccessKey) {
		const result =
			await $`op read "op://Engineering/rivet-releases R2 Upload/password"`;
		awsSecretAccessKey = result.stdout.trim();
	}

	assert(awsAccessKeyId, "AWS_ACCESS_KEY_ID is required");
	assert(awsSecretAccessKey, "AWS_SECRET_ACCESS_KEY is required");

	cachedConfig = {
		awsEnv: {
			AWS_ACCESS_KEY_ID: awsAccessKeyId,
			AWS_SECRET_ACCESS_KEY: awsSecretAccessKey,
			AWS_DEFAULT_REGION: "auto",
		},
		endpointUrl:
			"https://2a94c6a0ced8d35ea63cddc86c2681e7.r2.cloudflarestorage.com",
	};

	return cachedConfig;
}

export async function uploadDirToReleases(
	localPath: string,
	remotePath: string,
): Promise<void> {
	const { awsEnv, endpointUrl } = await getReleasesS3Config();
	await $({
		env: awsEnv,
		shell: true,
		stdio: "inherit",
	})`aws s3 sync ${localPath} s3://rivet-releases/${remotePath} --endpoint-url ${endpointUrl}`;
}

export async function uploadContentToReleases(
	content: string,
	remotePath: string,
): Promise<void> {
	const { awsEnv, endpointUrl } = await getReleasesS3Config();
	await $({
		env: awsEnv,
		input: content,
		shell: true,
		stdio: ["pipe", "inherit", "inherit"],
	})`aws s3 cp - s3://rivet-releases/${remotePath} --endpoint-url ${endpointUrl}`;
}

export interface ListReleasesResult {
	Contents?: { Key: string; Size: number }[];
}

export async function listReleasesObjects(
	prefix: string,
): Promise<ListReleasesResult> {
	const { awsEnv, endpointUrl } = await getReleasesS3Config();
	const result = await $({
		env: awsEnv,
		shell: true,
		stdio: ["pipe", "pipe", "inherit"],
	})`aws s3api list-objects --bucket rivet-releases --prefix ${prefix} --endpoint-url ${endpointUrl}`;
	return JSON.parse(result.stdout);
}

export async function deleteReleasesPath(remotePath: string): Promise<void> {
	const { awsEnv, endpointUrl } = await getReleasesS3Config();
	await $({
		env: awsEnv,
		shell: true,
		stdio: "inherit",
	})`aws s3 rm s3://rivet-releases/${remotePath} --recursive --endpoint-url ${endpointUrl}`;
}

export async function copyReleasesPath(
	sourcePath: string,
	targetPath: string,
): Promise<void> {
	const { awsEnv, endpointUrl } = await getReleasesS3Config();
	// Use sync instead of cp --recursive because R2 doesn't support the
	// x-amz-tagging-directive header that cp sends.
	await $({
		env: awsEnv,
		shell: true,
		stdio: "inherit",
	})`aws s3 sync s3://rivet-releases/${sourcePath} s3://rivet-releases/${targetPath} --endpoint-url ${endpointUrl}`;
}

export function assertEquals<T>(actual: T, expected: T, message?: string): void {
	if (actual !== expected) {
		throw new Error(message || `Expected ${expected}, got ${actual}`);
	}
}

export function assertExists<T>(
	value: T | null | undefined,
	message?: string,
): asserts value is T {
	if (value === null || value === undefined) {
		throw new Error(message || "Value does not exist");
	}
}

export async function assertDirExists(dirPath: string): Promise<void> {
	try {
		const stat = await fs.stat(dirPath);
		if (!stat.isDirectory()) {
			throw new Error(`Path exists but is not a directory: ${dirPath}`);
		}
	} catch (err: any) {
		if (err.code === "ENOENT") {
			throw new Error(`Directory not found: ${dirPath}`);
		}
		throw err;
	}
}
