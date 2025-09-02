#!/usr/bin/env -S deno run -A

import { assert } from "@std/assert";

const FERN_GROUP = Deno.env.get("FERN_GROUP");
if (!FERN_GROUP) throw "Missing FERN_GROUP";
const OPENAPI_PATH = `out/openapi.json`;
const GEN_PATH_RUST = `sdks/rust/api-${FERN_GROUP}/rust`;

async function generateRustSdk() {
	console.log("Running OpenAPI generator");

	// Delete existing directories
	await Deno.remove(GEN_PATH_RUST, { recursive: true }).catch(() => {});

	const dockerCmd = new Deno.Command("docker", {
		args: [
			"run",
			"--rm",
			`-u=${Deno.uid()}:${Deno.gid()}`,
			`-v=${Deno.cwd()}:/data`,
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
		],
		stdout: "inherit",
		stderr: "inherit",
	});
	const dockerResult = await dockerCmd.spawn().status;
	assert(dockerResult.success, "Docker command failed");
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
		let content;
		try {
			content = await Deno.readTextFile(filePath);
		} catch (error) {
			if (error instanceof Deno.errors.NotFound) {
				console.warn(`File not found: ${filePath}`);
				continue;
			} else {
				throw error;
			}
		}

		for (const [from, to] of replacements) {
			content = content.replace(from, to);
		}
		await Deno.writeTextFile(filePath, content);
	}
}

async function modifyDependencies() {
	// Remove reqwest's dependency on OpenSSL in favor of Rustls
	const cargoTomlPath = `${GEN_PATH_RUST}/Cargo.toml`;
	let cargoToml = await Deno.readTextFile(cargoTomlPath);
	cargoToml = cargoToml.replace(
		/\[dependencies\.reqwest\]/,
		"[dependencies.reqwest]\ndefault-features = false",
	);
	await Deno.writeTextFile(cargoTomlPath, cargoToml);
}

async function applyErrorPatch() {
	console.log("Applying error patch");

	// Improve the display printing of errors
	const modRsPath = `${GEN_PATH_RUST}/src/apis/mod.rs`;
	const patchFilePath = "./scripts/openapi/error.patch";
	const patchProcess = new Deno.Command("patch", {
		args: [modRsPath, patchFilePath],
		stdout: "inherit",
		stderr: "inherit",
	});
	const { success } = await patchProcess.spawn().status;
	assert(success, "Failed to apply patch");
}

async function formatSdk() {
	await new Deno.Command("cargo", { args: ["fmt"] }).output();
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

main();
