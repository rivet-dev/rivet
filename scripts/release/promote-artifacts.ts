import * as fs from "node:fs/promises";
import * as path from "node:path";
import { $ } from "execa";
import type { ReleaseOpts } from "./main";
import {
	copyReleasesPath,
	deleteReleasesPath,
	fetchGitRef,
	listReleasesObjects,
	uploadContentToReleases,
	versionOrCommitToRef,
} from "./utils";

export async function promoteArtifacts(opts: ReleaseOpts) {
	// Determine the S3 source prefix for engine artifacts. When reusing a
	// version, use the version-prefixed path (works even if that version
	// itself reused engine from an earlier release). When reusing a commit,
	// use the commit-prefixed path.
	let sourcePrefix: string;
	if (opts.reuseEngineVersion) {
		if (opts.reuseEngineVersion.includes(".")) {
			sourcePrefix = opts.reuseEngineVersion;
			console.log(`==> Reusing artifacts from version path: rivet/${sourcePrefix}/`);
		} else {
			console.log(`==> Reusing artifacts from commit ${opts.reuseEngineVersion}`);
			const ref = versionOrCommitToRef(opts.reuseEngineVersion);
			await fetchGitRef(ref);
			const result = await $`git rev-parse ${ref}`;
			sourcePrefix = result.stdout.trim().slice(0, 7);
			console.log(`==> Source commit: ${sourcePrefix}`);
		}
	} else {
		sourcePrefix = opts.commit;
	}

	// Promote engine artifacts (uploaded by CI in release.yaml to rivet/{commit}/engine/)
	await promotePath(opts, sourcePrefix, "engine");

	// Promote devtools artifacts (uploaded by build-artifacts.ts to rivet/{commit}/devtools/)
	await promotePath(opts, sourcePrefix, "devtools");

	// Upload install scripts
	await uploadInstallScripts(opts, opts.version);
	if (opts.latest) {
		await uploadInstallScripts(opts, "latest");
	}
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

async function copyPath(sourcePrefix: string, targetPrefix: string) {
	console.log(`Copying ${sourcePrefix} -> ${targetPrefix}`);
	await deleteReleasesPath(targetPrefix);
	await copyReleasesPath(sourcePrefix, targetPrefix);
}

/** S3-to-S3 copy from rivet/{commit}/{name}/ to rivet/{version}/{name}/ */
async function promotePath(opts: ReleaseOpts, sourceCommit: string, name: string) {
	console.log(`==> Promoting ${name} artifacts`);

	const sourcePrefix = `rivet/${sourceCommit}/${name}/`;
	const commitFiles = await listReleasesObjects(sourcePrefix);
	if (!Array.isArray(commitFiles?.Contents) || commitFiles.Contents.length === 0) {
		throw new Error(`No files found under ${sourcePrefix}`);
	}

	await copyPath(sourcePrefix, `rivet/${opts.version}/${name}/`);
	if (opts.latest) {
		await copyPath(sourcePrefix, `rivet/latest/${name}/`);
	}
}
