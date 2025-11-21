import * as fs from "node:fs/promises";
import * as path from "node:path";
import { $ } from "execa";
import type { ReleaseOpts } from "./main";
import {
	copyReleasesPath,
	deleteReleasesPath,
	listReleasesObjects,
	uploadContentToReleases,
} from "./utils";

export async function promoteArtifacts(opts: ReleaseOpts) {
	// Determine which commit to use for source artifacts
	let sourceCommit = opts.commit;
	if (opts.reuseEngineVersion) {
		console.log(`==> Reusing artifacts from version ${opts.reuseEngineVersion}`);
		console.log(`==> Fetching tags...`);
		await $({ stdio: "inherit" })`git fetch --tags`;
		const result = await $`git rev-parse v${opts.reuseEngineVersion}`;
		sourceCommit = result.stdout.trim().slice(0, 7);
		console.log(`==> Source commit: ${sourceCommit}`);
	}

	// Promote engine artifacts
	await promotePath(opts, sourceCommit, { name: "Engine", required: true });

	// Promote devtools artifacts
	await promotePath(opts, sourceCommit, { name: "DevTools", subPath: "devtools", required: false });

	// Upload install scripts
	await uploadInstallScripts(opts, opts.version);
	if (opts.latest) {
		await uploadInstallScripts(opts, "latest");
	}
}

interface PromoteOptions {
	name: string;
	subPath?: string;
	required: boolean;
}

async function promotePath(
	opts: ReleaseOpts,
	sourceCommit: string,
	{ name, subPath, required }: PromoteOptions,
) {
	console.log(`==> Promoting ${name} Artifacts`);

	const pathSuffix = subPath ? `/${subPath}` : "";
	const commitPrefix = `rivet/${sourceCommit}${pathSuffix}/`;

	console.log(`Checking for ${name.toLowerCase()} at ${commitPrefix}`);
	let commitFiles;
	try {
		commitFiles = await listReleasesObjects(commitPrefix);
	} catch {
		if (required) {
			throw new Error(`No files found under ${commitPrefix}`);
		}
		console.log(`⚠️  No ${name} artifacts found at ${commitPrefix}, skipping`);
		return;
	}

	if (!Array.isArray(commitFiles?.Contents) || commitFiles.Contents.length === 0) {
		if (required) {
			throw new Error(`No files found under ${commitPrefix}`);
		}
		console.log(`⚠️  No ${name} artifacts found at ${commitPrefix}, skipping`);
		return;
	}

	// Copy to version directory
	await copyPath(commitPrefix, `rivet/${opts.version}${pathSuffix}/`);

	// Copy to latest directory if applicable
	if (opts.latest) {
		await copyPath(commitPrefix, `rivet/latest${pathSuffix}/`);
	}

	console.log(`✅ ${name} artifacts promoted successfully`);
}

async function copyPath(sourcePrefix: string, targetPrefix: string) {
	console.log(`Copying ${sourcePrefix} -> ${targetPrefix}`);
	await deleteReleasesPath(targetPrefix);
	await copyReleasesPath(sourcePrefix, targetPrefix);
}

async function uploadInstallScripts(opts: ReleaseOpts, version: string) {
	const installScriptPaths = [
		path.resolve(opts.root, "scripts/release/static/install.sh"),
		path.resolve(opts.root, "scripts/release/static/install.ps1"),
	];

	for (const scriptPath of installScriptPaths) {
		let scriptContent = await fs.readFile(scriptPath, "utf-8");
		scriptContent = scriptContent.replace(/__VERSION__/g, version);

		const uploadKey = `rivet/${version}/${scriptPath.split("/").pop() ?? ""}`;

		console.log(`Uploading install script: ${uploadKey}`);
		await uploadContentToReleases(scriptContent, uploadKey);
	}
}
