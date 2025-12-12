#!/usr/bin/env tsx

import { spawn } from 'node:child_process';
import { rm, readFile, writeFile } from 'node:fs/promises';
import { promisify } from 'node:util';

const FERN_GROUP = process.env.FERN_GROUP;
if (!FERN_GROUP) throw new Error("Missing FERN_GROUP");
const OPENAPI_PATH = `engine/artifacts/openapi.json`;
const GEN_PATH_RUST = `engine/sdks/rust/api-${FERN_GROUP}/rust`;

function runCommand(command: string, args: string[]): Promise<void> {
	return new Promise((resolve, reject) => {
		const child = spawn(command, args, {
			stdio: 'inherit',
			cwd: process.cwd(),
		});

		child.on('exit', (code) => {
			if (code === 0) {
				resolve();
			} else {
				reject(new Error(`${command} exited with code ${code}`));
			}
		});

		child.on('error', reject);
	});
}

async function generateRustSdk() {
	console.log("Running OpenAPI generator");

	// Delete existing directories
	await rm(GEN_PATH_RUST, { recursive: true, force: true });

	const uid = process.getuid?.() ?? 1000;
	const gid = process.getgid?.() ?? 1000;

	await runCommand("docker", [
		"run",
		"--rm",
		`-u=${uid}:${gid}`,
		`-v=${process.cwd()}:/data`,
		"openapitools/openapi-generator-cli:v7.14.0",
		"generate",
		"-i",
		`/data/${OPENAPI_PATH}`,
		"--additional-properties=removeEnumValuePrefix=false",
		"-g",
		"rust",
		"-o",
		`/data/${GEN_PATH_RUST}`,
		"-p",
		`packageName=rivet-api-${FERN_GROUP}`,
	]);
}

async function fixOpenApiBugs() {
	const files: Record<string, [RegExp, string][]> = {
		//"cloud_games_matchmaker_api.rs": [
		//	[/CloudGamesLogStream/g, "crate::models::CloudGamesLogStream"],
		//],
		//"actors_api.rs": [
		//	[/ActorsEndpointType/g, "crate::models::ActorsEndpointType"],
		//],
		//"actors_logs_api.rs": [
		//	[/ActorsQueryLogStream/g, "crate::models::ActorsQueryLogStream"],
		//],
		//"containers_api.rs": [
		//	[/ContainersEndpointType/g, "crate::models::ContainersEndpointType"],
		//],
		//"containers_logs_api.rs": [
		//	[/ContainersQueryLogStream/g, "crate::models::ContainersQueryLogStream"],
		//],
		//"actors_v1_api.rs": [
		//	[/ActorsV1EndpointType/g, "crate::models::ActorsV1EndpointType"],
		//],
		//"actors_v1_logs_api.rs": [
		//	[/ActorsV1QueryLogStream/g, "crate::models::ActorsV1QueryLogStream"],
		//],
		//"servers_logs_api.rs": [
		//	[/ServersLogStream/g, "crate::models::ServersLogStream"],
		//],
	};

	for (const [file, replacements] of Object.entries(files)) {
		const filePath = `${GEN_PATH_RUST}/src/apis/${file}`;
		let content: string;
		try {
			content = await readFile(filePath, 'utf-8');
		} catch (error: any) {
			if (error.code === 'ENOENT') {
				console.warn(`File not found: ${filePath}`);
				continue;
			} else {
				throw error;
			}
		}

		for (const [from, to] of replacements) {
			content = content.replace(from, to);
		}
		await writeFile(filePath, content, 'utf-8');
	}
}

async function modifyDependencies() {
	// Remove reqwest's dependency on OpenSSL in favor of Rustls
	const cargoTomlPath = `${GEN_PATH_RUST}/Cargo.toml`;
	let cargoToml = await readFile(cargoTomlPath, 'utf-8');
	cargoToml = cargoToml.replace(
		/\[dependencies\.reqwest\]/,
		"[dependencies.reqwest]\ndefault-features = false",
	);
	await writeFile(cargoTomlPath, cargoToml, 'utf-8');
}

async function applyErrorPatch() {
	console.log("Applying error patch");

	// Improve the display printing of errors
	const modRsPath = `${GEN_PATH_RUST}/src/apis/mod.rs`;
	const patchFilePath = "./scripts/openapi/error.patch";

	await runCommand("patch", [modRsPath, patchFilePath]);
}

async function formatSdk() {
	await runCommand("cargo", ["fmt"]);
}

async function main() {
	await generateRustSdk();
	await fixOpenApiBugs();
	await modifyDependencies();
	await formatSdk(); // Format so patch is consistent
	// await applyErrorPatch();  // TODO: Broken
	await formatSdk(); // Format again after patched

	console.log("Done");
}

main().catch((error) => {
	console.error(error);
	process.exit(1);
});
