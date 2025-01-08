#!/usr/bin/env -S deno run -A

import { assert } from "jsr:@std/assert";
import { format } from "jsr:@std/datetime";
import { resolve } from "jsr:@std/path";
import { S3Bucket } from "https://deno.land/x/s3@0.5.0/mod.ts";

function getRequiredEnvVar(name: string): string {
	const value = Deno.env.get(name);
	if (!value) {
		throw new Error(`Required environment variable ${name} is not set`);
	}
	return value;
}

const awsAccessKeyId = getRequiredEnvVar("AWS_ACCESS_KEY_ID");
const awsSecretAccessKey = getRequiredEnvVar("AWS_SECRET_ACCESS_KEY");
const fontawesomePackageToken = getRequiredEnvVar("FONTAWESOME_PACKAGE_TOKEN");
const sentryAuthToken = getRequiredEnvVar("SENTRY_AUTH_TOKEN");
const assetsUrl = getRequiredEnvVar("ASSETS_URL");
const outerbaseProviderToken = getRequiredEnvVar("OUTERBASE_PROVIDER_TOKEN");
const posthogApiKey = getRequiredEnvVar("POSTHOG_API_KEY");
const posthogHost = getRequiredEnvVar("POSTHOG_HOST");
const sentryDsn = getRequiredEnvVar("SENTRY_DSN");
const sentryProjectId = getRequiredEnvVar("SENTRY_PROJECT_ID");

const DIST_DIR = resolve("apps/hub/dist");

async function getGitHash(): Promise<string> {
	const gitOutput = await new Deno.Command("git", {
		args: ["rev-parse", "--short", "HEAD"],
		stdout: "piped",
	}).output();
	assert(gitOutput.success, "Failed to get git hash");
	return new TextDecoder().decode(gitOutput.stdout).trim();
}

async function runBuild(baseUrl = "/") {
	console.log("Running yarn install");
	const installOutput = await new Deno.Command("yarn", {
		args: ["install"],
		stdout: "inherit",
		stderr: "inherit",
		env: {
			FONTAWESOME_PACKAGE_TOKEN: fontawesomePackageToken,
		},
	}).output();
	assert(installOutput.success, "Failed to run yarn install");

	console.log(
		`Running yarn build${baseUrl ? ` with base URL: ${baseUrl}` : ""}`,
	);
	const buildOutput = await new Deno.Command("yarn", {
		args: ["run", "build:prod", `--base=${baseUrl}`],
		stdout: "inherit",
		stderr: "inherit",
		env: {
			FONTAWESOME_PACKAGE_TOKEN: fontawesomePackageToken,
			SENTRY_AUTH_TOKEN: sentryAuthToken,
			VITE_APP_ASSETS_URL: assetsUrl,
			VITE_APP_OUTERBASE_PROVIDER_TOKEN: outerbaseProviderToken,
			VITE_APP_POSTHOG_API_KEY: posthogApiKey,
			VITE_APP_POSTHOG_HOST: posthogHost,
			VITE_APP_SENTRY_DSN: sentryDsn,
			VITE_APP_SENTRY_PROJECT_ID: sentryProjectId,
		},
	}).output();
	assert(buildOutput.success, "Failed to run yarn build");
}

async function generateZipFile(variant: string): Promise<string> {
	const gitHash = await getGitHash();
	const timestamp = format(new Date(), "yyyy-MM-dd-HH-mm-ss", {
		timeZone: "UTC",
	});
	const fileName = `${timestamp}-${gitHash}-${variant}.zip`;
	console.log(`Generating zip file: ${fileName}`);

	const tempDir = await Deno.makeTempDir();
	const zipPath = resolve(tempDir, fileName);

	const zipOutput = await new Deno.Command("zip", {
		args: ["-r", zipPath, "."],
		cwd: DIST_DIR,
		stdout: "inherit",
		stderr: "inherit",
	}).output();
	assert(zipOutput.success, "Failed to create zip file");
	console.log(`Zip file created: ${zipPath}`);
	return zipPath;
}

async function uploadZipToS3(zipPath: string): Promise<string> {
	console.log("Uploading zip file to S3");
	const bucket = new S3Bucket({
		accessKeyID: awsAccessKeyId,
		secretKey: awsSecretAccessKey,
		bucket: "rivet-releases",
		region: "auto",
		endpointURL:
			"https://2a94c6a0ced8d35ea63cddc86c2681e7.r2.cloudflarestorage.com/rivet-releases",
	});

	const fileName = zipPath.split("/").pop();
	if (!fileName) {
		throw new Error("Failed to extract file name from zip path");
	}
	const objectKey = `hub/${fileName}`;

	const zipFileData = await Deno.readFile(zipPath);
	await bucket.putObject(objectKey, zipFileData);

	const zipUrl = `https://releases.rivet.gg/${objectKey}`;
	console.log(`Uploaded zip file to S3: ${zipUrl}`);
	return zipUrl;
}

async function main() {
	// Build and upload the normal variant
	await runBuild();
	const normalZipPath = await generateZipFile("default");
	const normalZipUrl = await uploadZipToS3(normalZipPath);
	console.log("Uploaded normal variant zip URL:", normalZipUrl);

	// Build and upload the variant with VITE_BASE_URL="/ui/"
	await runBuild("/ui/");
	const uiBaseZipPath = await generateZipFile("embed");
	const uiBaseZipUrl = await uploadZipToS3(uiBaseZipPath);
	console.log("Uploaded UI base variant zip URL:", uiBaseZipUrl);
}

if (import.meta.main) {
	main();
}
