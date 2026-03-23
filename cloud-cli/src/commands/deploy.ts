/**
 * `rivet-cloud deploy` — build a Docker image, push it to the Rivet registry,
 * and update the managed pool configuration.
 *
 * Flow (mirrors rivet-dev/deploy-action):
 *   1. Inspect the cloud token → get org + project
 *   2. Ensure the target namespace exists (create if absent)
 *   3. Ensure the managed pool exists (create with zero replicas if absent)
 *   4. `docker login` to the Rivet registry using the cloud token
 *   5. `docker build` the local context
 *   6. `docker tag` + `docker push` the image
 *   7. `managedPools.upsert` with the new image reference
 */

import { type Rivet, RivetError } from "@rivet-gg/cloud";
import type { Command } from "commander";
import { attemptAsync, delay } from "es-toolkit";
import task from "tasuku";
import { resolveToken } from "../lib/auth.ts";
import { createCloudClient, type DockerCredentials } from "../lib/client.ts";
import {
	assertDockerAvailable,
	dockerBuild,
	dockerLogin,
	dockerTagAndPush,
} from "../lib/docker.ts";
import { colors, detail, fatal, header } from "../utils/output.ts";

export interface DeployOptions {
	token?: string;
	namespace: string;
	pool: string;
	context: string;
	dockerfile?: string;
	tag?: string;
	minCount: string;
	maxCount: string;
	env?: string[];
	command?: string;
	args?: string;
	apiUrl?: string;
	platform?: string;
	registryUrl?: string;
}

const IMAGE_ID_DISPLAY_LENGTH = 19;

export function registerDeployCommand(program: Command): void {
	program
		.command("deploy")
		.description(
			"Build a Docker image and deploy it to a Rivet Cloud managed pool",
		)
		.option(
			"-t, --token <token>",
			"Cloud API token (overrides RIVET_CLOUD_TOKEN)",
		)
		.option(
			"-n, --namespace <name>",
			"Target namespace (created if absent)",
			"production",
		)
		.option("-p, --pool <name>", "Managed pool name", "default")
		.option("--context <path>", "Docker build context directory", ".")
		.option("-f, --dockerfile <path>", "Path to Dockerfile")
		.option(
			"--tag <tag>",
			"Docker image tag (defaults to git short SHA or timestamp)",
		)
		.option("--min-count <n>", "Minimum runner instances", "1")
		.option("--max-count <n>", "Maximum runner instances", "10")
		.option(
			"-e, --env <KEY=VALUE>",
			"Environment variable (repeatable)",
			(val: string, prev: string[]) => [...(prev ?? []), val],
		)
		.option("--command <cmd>", "Override container entrypoint command")
		.option("--args <args>", "Space-separated args passed to the command")
		.option(
			"--platform <platform>",
			"Docker build platform (e.g. linux/amd64)",
			"linux/amd64",
		)
		.option(
			"--api-url <url>",
			"Cloud API base URL",
			"https://cloud-api.rivet.dev",
		)
		.option(
			"--registry-url <url>",
			"Docker registry URL",
			"registry.rivet.dev",
		)
		.action(async (opts: DeployOptions) => {
			await runDeploy(opts);
		});
}

async function runDeploy(opts: DeployOptions): Promise<void> {
	const token = resolveToken(opts.token);
	const client = createCloudClient({ token, baseUrl: opts.apiUrl });

	console.log(
		`\n${colors.accentBold("▶")} ${colors.label("Rivet Cloud Deploy")}\n`,
	);

	await task("Setting up", async ({ task, }) => {
		const { result: identity } = await task(
			"Authenticating",
			async ({ setTitle, setStatus }) => {
				const [error, identity] = await attemptAsync(() => client.apiTokens.inspect());

				if (!identity) {
					return fatal(
						"Authentication failed. Check that RIVET_CLOUD_TOKEN is valid and not expired.",
						error,
					);
				};

				setTitle(
					`Authenticated`,
				);
				setStatus(`${identity.organization} / ${identity.project}`)
				return identity;
			},
			{},
		);

		// Step 2: Ensure namespace exists
		const {
			result: { namespace },
		} = await task(`Namespace: ${opts.namespace}`, async ({ setTitle, setStatus }) => {
			try {
				const ns = await client.namespaces.get(identity.project, opts.namespace, {
					org: identity.organization,
				});
				setStatus("Exists");
				return ns;
			} catch (err) {
				if (err instanceof RivetError && err.statusCode !== 404) {
					throw err;
				}

				// TODO: If the namespace doesn't exist, we should search for any similarly named namespaces and ask the user to confirm before creating a new one. This can help prevent typos from creating unintended namespaces.
				// For now, we just create the namespace if it's not found.
				// In the future, we may want to add a `--force` flag to skip the confirmation prompt.
			}
			const ns = await client.namespaces.create(identity.project, {
				displayName: opts.namespace,
				org: identity.organization,
			});
			setTitle(`Namespace: ${ns.namespace.name}`);
			setStatus("Created");
			return ns;
		});

		// Step 3: Docker availability
		await assertDockerAvailable();

		// Step 4: Ensure managed pool exists
		const {
			result: { managedPool },
		} = await task(`Managed pool: ${opts.pool}`, async ({ setStatus }) => {
			try {
				const response = await client.managedPools.get(
					identity.project,
					namespace.name,
					opts.pool,
					{ org: identity.organization },
				);

				setStatus("Exists");
				return response;
			} catch (err) {
				if (err instanceof RivetError && err.statusCode !== 404) {
					throw err;
				}
			}

			const pool = await client.managedPools.upsert(
				identity.project,
				namespace.name,
				opts.pool,
				{
					org: identity.organization,
					displayName: opts.pool,
					minCount: Number(opts.minCount),
					maxCount: Number(opts.maxCount),
				},
			);

			while (true) {
				const { managedPool: pool } = await client.managedPools.get(
					identity.project,
					namespace.name,
					opts.pool,
					{ org: identity.organization },
				);
				if (pool.status === "ready") {
					break;
				}
				// capitalize first letter of status for display
				const status = pool.status.charAt(0).toUpperCase() + pool.status.slice(1);
				setStatus(`${status}...`);
				await delay(2000);
			}

			setStatus("Created");
			return pool;
		});



		// Use the cloud token directly as Docker registry credentials.
		// The Rivet registry accepts cloud tokens as the password.
		const creds: DockerCredentials = {
			registryUrl: opts.registryUrl ?? "registry.rivet.dev",
			username: "token",
			password: token,
		};

		// Step 4: Docker login
		await task(
			`Logging in to ${creds.registryUrl}`,
			async ({ streamPreview }) => {
				await dockerLogin({ ...creds, stream: streamPreview });
			},
		);

	});

	// Step 5: Build Docker image
	const { result: buildResult } = await task(
		"Building Docker image",
		async ({ setTitle, streamPreview }) => {
			const result = await dockerBuild({
				context: opts.context,
				dockerfile: opts.dockerfile,
				platform: opts.platform,
				stream: streamPreview,
			});
			setTitle(
				`Built image ${result.imageId.slice(0, IMAGE_ID_DISPLAY_LENGTH)}`,
			);
			return result;
		},
	);

	// Derive image tag
	const imageTag = opts.tag ?? (await resolveImageTag());
	const repository = `${identity.project}/${opts.pool}`;
	const remoteRef = `${creds.registryUrl}/${identity.organization}/${repository}:${imageTag}`;

	// Step 6: Tag and push
	await task(`Pushing ${remoteRef}`, async ({ streamPreview }) => {
		await dockerTagAndPush({
			localImageId: buildResult.imageId,
			remoteRef,
			stream: streamPreview,
		});
	});

	// Step 7: Parse environment variables
	const envVars = parseEnvVars(opts.env);

	// Step 8: Upsert managed pool
	await task(
		`Updating managed pool "${managedPool.name}"`,
		async ({ setTitle }) => {
			await client.managedPools.upsert(
				identity.project,
				namespace.name,
				managedPool.name,
				{
					org: identity.organization,
					image: { repository, tag: imageTag },
					minCount: Number(opts.minCount),
					maxCount: Number(opts.maxCount),
					...(Object.keys(envVars).length > 0
						? { environment: envVars }
						: {}),
					...(opts.command ? { command: opts.command } : {}),
					...(opts.args ? { args: opts.args.split(" ") } : {}),
				},
			);
			setTitle(`Pool "${managedPool.name}" updated`);
		},
	);

	await task("Deploying", async ({ setStatus, setTitle }) => {
		while (true) {
			const { managedPool: pool } = await client.managedPools.get(
				identity.project,
				namespace.name,
				managedPool.name,
				{ org: identity.organization },
			);
			if (pool.error || pool.status === "ready") {
				break;
			}

			setStatus(pool.status);
			await delay(2000);
		}
		setTitle("Deployed!");
	});

	console.log("");
	detail("Namespace", opts.namespace);
	detail("Pool", managedPool.name);
	detail("Image", `${repository}:${imageTag}`);
	detail(
		"Dashboard",
		`https://dashboard.rivet.dev/orgs/${identity.organization}/projects/${identity.project}/ns/${opts.namespace}`,
	);
	console.log("");
}

async function resolveImageTag(): Promise<string> {
	try {
		const result = await Bun.$`git rev-parse --short HEAD`.quiet().text();
		return result.trim();
	} catch {
		return `deploy-${Date.now()}`;
	}
}

function parseEnvVars(raw: string[] | undefined): Record<string, string> {
	if (!raw?.length) return {};
	const out: Record<string, string> = {};
	for (const entry of raw) {
		const eq = entry.indexOf("=");
		if (eq === -1) {
			fatal(`Invalid --env value "${entry}". Expected KEY=VALUE format.`);
		}
		const key = entry.slice(0, eq);
		const value = entry.slice(eq + 1);
		out[key] = value;
	}
	return out;
}
