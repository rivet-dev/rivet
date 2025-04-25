#!/usr/bin/env -S deno run -A

import { parse, stringify } from "@std/yaml";
import { assert } from "@std/assert";

const FERN_GROUP = Deno.env.get("FERN_GROUP");
const SDKS_PATH = `sdks/api/${FERN_GROUP}`;
const GEN_PATH_OPENAPI = `${SDKS_PATH}/openapi_compat/openapi.yml`;
const GEN_PATH_RUST = `${SDKS_PATH}/rust`;

async function createOutputDir(outputDir: string) {
	console.log("Creating output dir");
	try {
		await Deno.mkdir(outputDir, { recursive: true });
	} catch (error) {
		if (!(error instanceof Deno.errors.AlreadyExists)) {
			throw error;
		}
	}
}

async function modifyOpenApiSpec(specPath: string, outputPath: string) {
	console.log("Reading spec");
	const specContent = await Deno.readTextFile(specPath);
	const openapi = parse(specContent) as { info: { version: string } };

	console.log("Modifying spec for compatibility");
	openapi.info.version = "0.0.1";

	console.log("Writing new spec");
	await Deno.writeTextFile(outputPath, stringify(openapi));
}

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
			"openapitools/openapi-generator-cli:v6.4.0",
			"generate",
			"-i",
			`/data/${GEN_PATH_OPENAPI}`,
			"--additional-properties=removeEnumValuePrefix=false",
			"-g",
			"rust",
			"-o",
			`/data/${GEN_PATH_RUST}`,
			"-p",
			"packageName=rivet-api",
		],
		stdout: "inherit",
		stderr: "inherit",
	});
	const dockerResult = await dockerCmd.spawn().status;
	assert(dockerResult.success, "Docker command failed");
}

async function fixOpenApiBugs() {
	const files: Record<string, [RegExp, string][]> = {
		"cloud_games_matchmaker_api.rs": [
			[/CloudGamesLogStream/g, "crate::models::CloudGamesLogStream"],
		],
		"actors_api.rs": [
			[/ActorsEndpointType/g, "crate::models::ActorsEndpointType"],
		],
		"actors_logs_api.rs": [
			[/ActorsQueryLogStream/g, "crate::models::ActorsQueryLogStream"],
		],
		"servers_logs_api.rs": [
			[/ServersLogStream/g, "crate::models::ServersLogStream"],
		],
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
	const outputDir = `sdks/api/${FERN_GROUP}/openapi_compat`;
	const specPath = `sdks/api/${FERN_GROUP}/openapi/openapi.yml`;

	await createOutputDir(outputDir);
	await modifyOpenApiSpec(specPath, `${outputDir}/openapi.yml`);
	await generateRustSdk();
	await fixOpenApiBugs();
	await modifyDependencies();
	await formatSdk(); // Format so patch is consistent
	await applyErrorPatch();
	await formatSdk(); // Format again after patched

	console.log("Done");
}

main();
