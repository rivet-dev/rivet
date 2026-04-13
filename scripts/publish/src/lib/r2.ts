/**
 * R2 client + upload + copy helpers for the Rivet releases bucket.
 *
 * Credentials come from env vars in CI (`R2_RELEASES_ACCESS_KEY_ID` +
 * `R2_RELEASES_SECRET_ACCESS_KEY`). When running locally (not in GitHub
 * Actions), falls back to 1Password via the `op` CLI. The fallback is gated
 * on `GITHUB_ACTIONS !== "true"` specifically — not a generic `CI` check —
 * so it doesn't accidentally trigger inside other CI systems or laptops that
 * export `CI=true`.
 *
 * Implementation note: we use `aws s3 cp` / `aws s3api copy-object` shelling
 * out because Cloudflare R2 does not support the `x-amz-tagging-directive`
 * header that the AWS SDK sends even with `--copy-props none`. The `s3api
 * copy-object` path avoids it. See:
 *   https://community.cloudflare.com/t/r2-s3-compat-doesnt-support-net-sdk-for-copy-operations-due-to-tagging-header/616867
 */
import * as fs from "node:fs/promises";
import { $ } from "execa";
import { scoped } from "./logger.js";

const log = scoped("r2");

const BUCKET = "rivet-releases";
const ENDPOINT_URL =
	"https://2a94c6a0ced8d35ea63cddc86c2681e7.r2.cloudflarestorage.com";

type R2Env = Record<string, string>;

let cached: R2Env | null = null;

async function getR2Env(): Promise<R2Env> {
	if (cached) return cached;
	let ak = process.env.R2_RELEASES_ACCESS_KEY_ID;
	let sk = process.env.R2_RELEASES_SECRET_ACCESS_KEY;

	// Only fall back to 1Password outside GitHub Actions — avoids confusing
	// failures when CI env vars are misconfigured.
	const inGithubActions = process.env.GITHUB_ACTIONS === "true";
	if (!ak && !inGithubActions) {
		const { stdout } = await $`op read ${"op://Engineering/rivet-releases R2 Upload/username"}`;
		ak = stdout.trim();
	}
	if (!sk && !inGithubActions) {
		const { stdout } = await $`op read ${"op://Engineering/rivet-releases R2 Upload/password"}`;
		sk = stdout.trim();
	}

	if (!ak || !sk) {
		throw new Error(
			"R2_RELEASES_ACCESS_KEY_ID and R2_RELEASES_SECRET_ACCESS_KEY must be set",
		);
	}
	cached = {
		AWS_ACCESS_KEY_ID: ak,
		AWS_SECRET_ACCESS_KEY: sk,
		AWS_DEFAULT_REGION: "auto",
	};
	return cached;
}

export interface ListEntry {
	Key: string;
	Size?: number;
}
export interface ListResult {
	Contents: ListEntry[];
}

export async function listObjects(prefix: string): Promise<ListResult> {
	const env = await getR2Env();
	const contents: ListEntry[] = [];
	let continuationToken: string | undefined;

	while (true) {
		const { stdout } = continuationToken
			? await $({
					env,
				})`aws s3api list-objects-v2 --bucket ${BUCKET} --prefix ${prefix} --continuation-token ${continuationToken} --endpoint-url ${ENDPOINT_URL}`
			: await $({
					env,
				})`aws s3api list-objects-v2 --bucket ${BUCKET} --prefix ${prefix} --endpoint-url ${ENDPOINT_URL}`;
		if (!stdout.trim()) break;

		const page = JSON.parse(stdout) as {
			Contents?: ListEntry[];
			IsTruncated?: boolean;
			NextContinuationToken?: string;
		};
		if (Array.isArray(page.Contents)) {
			contents.push(...page.Contents);
		}
		if (!page.IsTruncated || !page.NextContinuationToken) {
			break;
		}
		continuationToken = page.NextContinuationToken;
	}

	return { Contents: contents };
}

/** Upload a single file to R2. */
export async function uploadFile(
	localPath: string,
	r2Key: string,
): Promise<void> {
	const env = await getR2Env();
	log.info(`uploading ${localPath} -> ${r2Key}`);
	await $({
		env,
		stdio: "inherit",
	})`aws s3 cp ${localPath} s3://${BUCKET}/${r2Key} --endpoint-url ${ENDPOINT_URL} --checksum-algorithm CRC32`;
}

/** Recursively upload a directory to an R2 prefix. */
export async function uploadDir(
	localDir: string,
	r2Prefix: string,
): Promise<void> {
	const env = await getR2Env();
	log.info(`uploading directory ${localDir} -> ${r2Prefix}`);
	await $({
		env,
		stdio: "inherit",
	})`aws s3 cp ${localDir} s3://${BUCKET}/${r2Prefix} --recursive --endpoint-url ${ENDPOINT_URL} --checksum-algorithm CRC32`;
}

/** Upload a string as a single object (used for install scripts). */
export async function uploadString(
	content: string,
	r2Key: string,
): Promise<void> {
	const env = await getR2Env();
	log.info(`uploading content -> ${r2Key}`);
	await $({
		env,
		input: content,
		stdio: ["pipe", "inherit", "inherit"],
	})`aws s3 cp - s3://${BUCKET}/${r2Key} --endpoint-url ${ENDPOINT_URL}`;
}

/** Delete every object under an R2 prefix. */
export async function deletePrefix(r2Prefix: string): Promise<void> {
	const env = await getR2Env();
	log.info(`deleting ${r2Prefix}`);
	await $({
		env,
		stdio: "inherit",
	})`aws s3 rm s3://${BUCKET}/${r2Prefix} --recursive --endpoint-url ${ENDPOINT_URL}`;
}

/**
 * Copy every object under `sourcePrefix` to `targetPrefix`. Uses `s3api
 * copy-object` per-object to avoid the R2 tagging-directive bug.
 */
export async function copyPrefix(
	sourcePrefix: string,
	targetPrefix: string,
): Promise<void> {
	const env = await getR2Env();
	log.info(`copying ${sourcePrefix} -> ${targetPrefix}`);

	const list = await listObjects(sourcePrefix);
	if (list.Contents.length === 0) {
		log.warn(
			`source prefix ${sourcePrefix} is empty. Skipping copy to ${targetPrefix}.`,
		);
		return;
	}

	// Delete the target first so stale files from a prior publish are cleaned.
	try {
		await deletePrefix(targetPrefix);
	} catch {
		// Target may not exist yet — that's fine.
	}

	for (const obj of list.Contents) {
		const sourceKey = obj.Key;
		const targetKey = sourceKey.replace(sourcePrefix, targetPrefix);
		log.info(`  ${sourceKey} -> ${targetKey}`);
		await $({
			env,
		})`aws s3api copy-object --bucket ${BUCKET} --key ${targetKey} --copy-source ${BUCKET}/${sourceKey} --endpoint-url ${ENDPOINT_URL}`;
	}
}

/**
 * Upload install scripts under a version prefix. Templates `__VERSION__` to
 * the given version so `curl | sh` scripts point at the right release.
 */
export async function uploadInstallScripts(
	installScriptPaths: string[],
	version: string,
): Promise<void> {
	for (const scriptPath of installScriptPaths) {
		let content = await fs.readFile(scriptPath, "utf-8");
		content = content.replace(/__VERSION__/g, version);
		const fileName = scriptPath.split("/").pop() ?? "";
		const key = `rivet/${version}/${fileName}`;
		log.info(`uploading install script ${key}`);
		await uploadString(content, key);
	}
}
