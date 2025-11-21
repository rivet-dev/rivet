import * as fs from "node:fs/promises";
import * as path from "node:path";
import { $ } from "execa";
import type { ReleaseOpts } from "./main";

function assert(condition: any, message?: string): asserts condition {
	if (!condition) {
		throw new Error(message || "Assertion failed");
	}
}

export async function updateArtifacts(opts: ReleaseOpts) {
	// Get credentials and set them in the environment
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

	const endpointUrl =
		"https://2a94c6a0ced8d35ea63cddc86c2681e7.r2.cloudflarestorage.com";

	// Create AWS environment for commands
	const awsEnv: Record<string, string> = {
		AWS_ACCESS_KEY_ID: awsAccessKeyId,
		AWS_SECRET_ACCESS_KEY: awsSecretAccessKey,
		AWS_DEFAULT_REGION: "auto",
	};

	// Determine which commit to use for source artifacts
	let sourceCommit = opts.commit;
	if (opts.reuseEngineVersion) {
		console.log(`==> Reusing artifacts from version ${opts.reuseEngineVersion}`);
		// Fetch tags to ensure we have the version tag
		console.log(`==> Fetching tags...`);
		await $({ stdio: "inherit" })`git fetch --tags`;
		const result = await $`git rev-parse v${opts.reuseEngineVersion}`;
		sourceCommit = result.stdout.trim().slice(0, 7);
		console.log(`==> Source commit: ${sourceCommit}`);
	}

	// List all files under engine/{commit}/
	const commitPrefix = `engine/${sourceCommit}/`;
	console.log(`==> Listing Original Files: ${commitPrefix}`);
	const listResult = await $({
		env: awsEnv,
		shell: true,
		stdio: ["pipe", "pipe", "inherit"],
	})`aws s3api list-objects --bucket rivet-releases --prefix ${commitPrefix} --endpoint-url ${endpointUrl}`;
	const commitFiles = JSON.parse(listResult.stdout);
	assert(
		Array.isArray(commitFiles?.Contents) && commitFiles.Contents.length > 0,
		`No files found under engine/${sourceCommit}/`,
	);

	// Copy files to version directory
	const versionTarget = `engine/${opts.version}/`;
	await copyFiles(awsEnv, commitPrefix, versionTarget, endpointUrl);
	await generateInstallScripts(awsEnv, opts, opts.version, endpointUrl);

	// If this is the latest version, copy to latest directory
	if (opts.latest) {
		await copyFiles(awsEnv, commitPrefix, "engine/latest/", endpointUrl);
		await generateInstallScripts(awsEnv, opts, "latest", endpointUrl);
	}

	// Upload devtools artifacts
	await uploadDevtoolsArtifacts(awsEnv, opts, endpointUrl);
}

async function copyFiles(
	awsEnv: Record<string, string>,
	sourcePrefix: string,
	targetPrefix: string,
	endpointUrl: string,
) {
	console.log(`==> Copying Files: ${targetPrefix}`);
	// Delete existing files in target directory using --recursive
	console.log(`Deleting existing files in ${targetPrefix}`);
	await $({
		env: awsEnv,
		shell: true,
		stdio: "inherit",
	})`aws s3 rm s3://rivet-releases/${targetPrefix} --recursive --endpoint-url ${endpointUrl}`;

	// Copy new files using --recursive
	console.log(`Copying files from ${sourcePrefix} to ${targetPrefix}`);
	await $({
		env: awsEnv,
		shell: true,
		stdio: "inherit",
	})`aws s3 cp s3://rivet-releases/${sourcePrefix} s3://rivet-releases/${targetPrefix} --recursive --copy-props none --endpoint-url ${endpointUrl}`;
}

async function generateInstallScripts(
	awsEnv: Record<string, string>,
	opts: ReleaseOpts,
	version: string,
	endpointUrl: string,
) {
	const installScriptPaths = [
		path.resolve(opts.root, "scripts/release/static/install.sh"),
		path.resolve(opts.root, "scripts/release/static/install.ps1"),
	];

	for (const scriptPath of installScriptPaths) {
		let scriptContent = await fs.readFile(scriptPath, "utf-8");
		scriptContent = scriptContent.replace(/__VERSION__/g, version);

		const uploadKey = `engine/${version}/${scriptPath.split("/").pop() ?? ""}`;

		// Upload the install script to S3
		console.log(`==> Uploading Install Script: ${uploadKey}`);
		await $({
			env: awsEnv,
			input: scriptContent,
			shell: true,
			stdio: ["pipe", "inherit", "inherit"],
		})`aws s3 cp - s3://rivet-releases/${uploadKey} --endpoint-url ${endpointUrl}`;
	}
}

async function uploadDevtoolsArtifacts(
	awsEnv: Record<string, string>,
	opts: ReleaseOpts,
	endpointUrl: string,
) {
	console.log(`==> Uploading DevTools Artifacts`);

	const devtoolsDistPath = path.resolve(
		opts.root,
		"rivetkit-typescript/packages/devtools/dist",
	);

	// Check if devtools dist directory exists
	try {
		await fs.access(devtoolsDistPath);
	} catch {
		console.log(
			`⚠️  DevTools dist directory not found at ${devtoolsDistPath}, skipping`,
		);
		return;
	}

	// Upload to version directory
	const versionTarget = `devtools/${opts.version}/`;
	console.log(`Uploading devtools to ${versionTarget}`);
	await $({
		env: awsEnv,
		shell: true,
		stdio: "inherit",
	})`aws s3 sync ${devtoolsDistPath} s3://rivet-releases/${versionTarget} --endpoint-url ${endpointUrl}`;

	// If this is the latest version, also upload to latest directory
	if (opts.latest) {
		const latestTarget = "devtools/latest/";
		console.log(`Uploading devtools to ${latestTarget}`);
		await $({
			env: awsEnv,
			shell: true,
			stdio: "inherit",
		})`aws s3 sync ${devtoolsDistPath} s3://rivet-releases/${latestTarget} --endpoint-url ${endpointUrl}`;
	}

	console.log(`✅ DevTools artifacts uploaded successfully`);
}
