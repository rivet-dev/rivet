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

/**
 * Fetches a git ref from the remote. For tags, fetches all tags. For commits, unshallows the repo.
 */
export async function fetchGitRef(ref: string): Promise<void> {
	if (ref.startsWith("v")) {
		console.log(`Fetching tags...`);
		await $({ stdio: "inherit" })`git fetch --tags --force`;
	} else {
		// Git doesn't allow fetching commits directly by SHA, and CI often uses
		// shallow clones. Unshallow the repo to ensure the commit is available.
		console.log(`Unshallowing repo to find commit ${ref}...`);
		try {
			await $({ stdio: "inherit" })`git fetch --unshallow origin`;
		} catch {
			// Already unshallowed, just fetch
			await $({ stdio: "inherit" })`git fetch origin`;
		}
	}
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
			await $`op read ${"op://Engineering/rivet-releases R2 Upload/username"}`;
		awsAccessKeyId = result.stdout.trim();
	}
	let awsSecretAccessKey = process.env.R2_RELEASES_SECRET_ACCESS_KEY;
	if (!awsSecretAccessKey) {
		const result =
			await $`op read ${"op://Engineering/rivet-releases R2 Upload/password"}`;
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
	// Use --checksum-algorithm CRC32 for R2 compatibility (matches CI upload in release.yaml)
	await $({
		env: awsEnv,
		shell: true,
		stdio: "inherit",
	})`aws s3 cp ${localPath} s3://rivet-releases/${remotePath} --recursive --checksum-algorithm CRC32 --endpoint-url ${endpointUrl}`;
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

/**
 * Copies objects from one S3 path to another within the releases bucket.
 *
 * NOTE: We implement our own recursive copy instead of using `aws s3 cp --recursive`
 * because of a Cloudflare R2 bug. R2 doesn't support the `x-amz-tagging-directive`
 * header, which the AWS CLI sends even with `--copy-props none` for small files.
 * Using `s3api copy-object` directly avoids this header.
 *
 * See: https://community.cloudflare.com/t/r2-s3-compat-doesnt-support-net-sdk-for-copy-operations-due-to-tagging-header/616867
 */
export async function copyReleasesPath(
	sourcePath: string,
	targetPath: string,
): Promise<void> {
	const { awsEnv, endpointUrl } = await getReleasesS3Config();

	const listResult = await $({
		env: awsEnv,
	})`aws s3api list-objects --bucket rivet-releases --prefix ${sourcePath} --endpoint-url ${endpointUrl}`;

	const objects = JSON.parse(listResult.stdout);
	if (!objects.Contents?.length) {
		throw new Error(`No objects found under ${sourcePath}`);
	}

	for (const obj of objects.Contents) {
		const sourceKey = obj.Key;
		const targetKey = sourceKey.replace(sourcePath, targetPath);
		console.log(`  ${sourceKey} -> ${targetKey}`);
		await $({
			env: awsEnv,
		})`aws s3api copy-object --bucket rivet-releases --key ${targetKey} --copy-source rivet-releases/${sourceKey} --endpoint-url ${endpointUrl}`;
	}
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
