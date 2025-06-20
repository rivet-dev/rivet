import { Hono } from "hono";
import { exec } from "node:child_process";
import { promisify } from "node:util";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import temp from "temp";

const execAsync = promisify(exec);

// Auto-track and cleanup temp directories/files
temp.track();

// Config
const RIVET_CLOUD_TOKEN = process.env.RIVET_CLOUD_TOKEN;
const RIVET_PROJECT = process.env.RIVET_PROJECT;
const RIVET_ENVIRONMENT = process.env.RIVET_ENVIRONMENT;

if (!RIVET_CLOUD_TOKEN || !RIVET_PROJECT || !RIVET_ENVIRONMENT) {
	throw new Error(
		"Missing required environment variables: RIVET_CLOUD_TOKEN, RIVET_PROJECT, RIVET_ENVIRONMENT",
	);
}

export const app = new Hono();

app.onError((err, c) => {
	console.error("Error during operation:", err);
	return c.json(
		{
			error: "Operation failed",
			message: err instanceof Error ? err.message : String(err),
		},
		500,
	);
});

app.get("/", (c) => {
	return c.text("Multitenant Deploy Example");
});

app.post("/deploy/:appId", async (c) => {
	const appId = c.req.param("appId");

	// Get the form data
	const formData = await c.req.formData();

	if (!appId || typeof appId !== "string") {
		return c.json({ error: "Missing or invalid appId" }, 400);
	}

	// Validate app ID (alphanumeric and hyphens only, 3-30 chars)
	if (!/^[a-z0-9-]{3,30}$/.test(appId)) {
		return c.json(
			{
				error: "Invalid appId format. Must be 3-30 characters, lowercase alphanumeric with hyphens.",
			},
			400,
		);
	}

	// Create a temp directory for the files
	const tempDir = await temp.mkdir("rivet-deploy-");
	const tempDirProject = path.join(tempDir, "project");

	// Process and save each file
	let hasDockerfile = false;
	for (const [fieldName, value] of formData.entries()) {
		// Skip non-file fields
		if (!(value instanceof File)) continue;

		const filePath = path.join(tempDirProject, fieldName);

		await fs.mkdir(path.dirname(filePath), { recursive: true });

		await fs.writeFile(filePath, Buffer.from(await value.arrayBuffer()));

		if (fieldName === "Dockerfile") {
			hasDockerfile = true;
		}
	}

	if (!hasDockerfile) {
		return c.json({ error: "Dockerfile is required" }, 400);
	}

	// Tags unique to this app's functions
	const appTags = {
		// Specifies that this app is deployed by a user
		type: "user-app",
		// Specifies which app this function belongs to
		//
		// Used for attributing billing & more
		app: appId,
	};

	// Write Rivet config
	const functionName = `fn-${appId}`;
	const rivetConfig = {
		functions: {
			[functionName]: {
				build_path: "./project/",
				dockerfile: "./Dockerfile",
				unstable: {
					build_method: "remote"
				},
				build_args: {
					// See MY_ENV_VAR build args in Dockerfile
					MY_ENV_VAR: "custom env var",
					APP_ID: appId,
				},
				tags: appTags,
				route_subpaths: true,
				strip_prefix: true,
				resources: { cpu: 125, memory: 128 },
				// If you want to host at a subpath:
				// path: "/foobar"
			},
		},
	};
	await fs.writeFile(
		path.join(tempDir, "rivet.json"),
		JSON.stringify(rivetConfig),
	);

	// Run rivet publish command
	console.log(`Deploying app ${appId} from ${tempDir}...`);

	// Run the deploy command
	const deployResult = await execAsync(
		`rivet deploy --environment ${RIVET_ENVIRONMENT} --non-interactive`,
		{
			cwd: tempDir,
		},
	);

	console.log("Publish output:", deployResult.stdout);

	// Get the function endpoint
	const endpointResult = await execAsync(
		`rivet function endpoint --environment ${RIVET_ENVIRONMENT} ${functionName}`,
		{
			cwd: tempDir,
		},
	);

	// Strip any extra text and just get the URL
	const endpointUrl = endpointResult.stdout.trim();
	console.log("Function endpoint:", endpointUrl);

	return c.json({
		success: true,
		appId,
		endpoint: endpointUrl,
		buildOutput: deployResult.stdout,
	});
});
