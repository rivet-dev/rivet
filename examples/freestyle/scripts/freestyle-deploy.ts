import { type Rivet, RivetClient } from "@rivetkit/engine-api-full";
import { execSync } from "child_process";
import dotenv from "dotenv";
import { FreestyleSandboxes } from "freestyle-sandboxes";
import { prepareDirForDeploymentSync } from "freestyle-sandboxes/utils";
import { readFileSync } from "fs";

dotenv.config({ path: new URL("../.env", import.meta.url).pathname });

const FREESTYLE_DOMAIN = getEnv("FREESTYLE_DOMAIN");
const FREESTYLE_API_KEY = getEnv("FREESTYLE_API_KEY");
const RIVET_ENDPOINT = getEnv("RIVET_ENDPOINT");
const RIVET_TOKEN = getEnv("RIVET_TOKEN");
const RIVET_NAMESPACE_NAME = getEnv("RIVET_NAMESPACE");
const RIVET_RUNNER_NAME = "freestyle";

function getEnv(key: string): string {
	const value = process.env[key];
	if (typeof value === "undefined") {
		throw new Error(`Missing env var: ${key}`);
	}
	return value;
}

const rivet = new RivetClient({
	environment: RIVET_ENDPOINT,
	token: RIVET_TOKEN,
});

const freestyle = new FreestyleSandboxes({
	apiKey: FREESTYLE_API_KEY,
});

async function main() {
	const namespace = await getOrCreateNamespace({
		displayName: RIVET_NAMESPACE_NAME,
		name: RIVET_NAMESPACE_NAME,
	});
	console.log("Got namespace " + namespace.name);

	runBuildSteps();

	await deployToFreestyle();
	console.log("Deployed to freestyle");

	await updateRunnerConfig(namespace);
	console.log("Updated runner config");

	console.log("🎉 Deployment complete! 🎉");
	console.log(
		"Visit https://" +
			FREESTYLE_DOMAIN +
			"/ to see your frontend, which is connected to Rivet",
	);
}

function runBuildSteps() {
	console.log("Running build steps...");

	console.log("- Running vite build");
	execSync("vite build", {
		stdio: "inherit",
		env: {
			...process.env,
			VITE_RIVET_ENDPOINT: RIVET_ENDPOINT,
			VITE_RIVET_NAMESPACE: RIVET_NAMESPACE_NAME,
			VITE_RIVET_RUNNER_NAME: RIVET_RUNNER_NAME,
		},
	});

	console.log("- Running tsup");
	execSync("tsup", {
		stdio: "inherit",
	});

	console.log("Build complete!");
}

async function getOrCreateNamespace({
	name,
	displayName,
}: {
	name: string;
	displayName: string;
}): Promise<Rivet.Namespace> {
	console.log("- Checking for existing " + name + " namespace");
	const { namespaces } = await rivet.namespaces.list({
		limit: 32,
	});
	const existing = namespaces.find((ns) => ns.name === name);
	if (existing) {
		console.log("- Found existing namespace " + name);
		return existing;
	}
	console.log("- Creating namespace " + name);
	const { namespace } = await rivet.namespaces.create({
		displayName,
		name,
	});
	return namespace;
}

async function updateRunnerConfig(namespace: Rivet.Namespace) {
	console.log("- Updating runner config for " + RIVET_RUNNER_NAME);
	await rivet.runnerConfigs.upsert(RIVET_RUNNER_NAME, {
		serverless: {
			url: "https://" + FREESTYLE_DOMAIN + "/api/start",
			headers: {},
			runnersMargin: 1,
			minRunners: 1,
			maxRunners: 1,
			slotsPerRunner: 1,
			requestLifespan: 60 * 4 + 30,
		},
		namespace: namespace.name,
	});
}

async function deployToFreestyle() {
	console.log("- Deploying to freestyle at https://" + FREESTYLE_DOMAIN);

	const buildDir = prepareDirForDeploymentSync(
		new URL("../dist", import.meta.url).pathname,
	);
	if (buildDir.kind === "files") {
		buildDir.files["deno.json"] = {
			// Fix imports for Deno
			content: readFileSync(
				new URL("../deno.json", import.meta.url).pathname,
				"utf-8",
			),
			encoding: "utf-8",
		};
	} else {
		throw new Error("Expected buildDir to be files");
	}
	const res = await freestyle.deployWeb(buildDir, {
		envVars: {
			LOG_LEVEL: "debug",
			FREESTYLE_ENDPOINT: `https://${FREESTYLE_DOMAIN}`,
			RIVET_ENDPOINT,
			RIVET_RUNNER_KIND: "serverless",
		},
		timeout: 60 * 5,
		entrypoint: "server.cjs",
		domains: [FREESTYLE_DOMAIN],
		build: false,
	});

	console.log("Deployment id=" + res.deploymentId);
}

main();
