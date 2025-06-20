import { RivetClient } from "@rivet-gg/api";
import { BuildStore } from "../build-store";
import { serializeKanikoArguments } from "../common";

export async function runRivetBuild(
	buildStore: BuildStore,
	serverUrl: string,
	buildId: string,
): Promise<void> {
	const token = process.env.RIVET_CLOUD_TOKEN;
	if (!token) {
		throw new Error("RIVET_CLOUD_TOKEN environment variable is required");
	}

	const projectId = process.env.RIVET_PROJECT!;
	if (!projectId) {
		throw new Error("RIVET_PROJECT environment variable is required");
	}

	const environmentName = process.env.RIVET_ENVIRONMENT!;
	if (!environmentName) {
		throw new Error("RIVET_ENVIRONMENT environment variable is required");
	}

	const kanikoBuildId = process.env.KANIKO_BUILD_ID!;
	if (!kanikoBuildId) {
		throw new Error("KANIKO_BUILD_ID environment variable is required");
	}

	const client = new RivetClient({ token });

	const build = buildStore.getBuild(buildId);
	if (!build) {
		throw new Error(`Build ${buildId} not found`);
	}

	const contextUrl = `${serverUrl}/builds/${buildId}/kaniko/context.tar.gz`;
	const outputUrl = `${serverUrl}/builds/${buildId}/kaniko/output.tar.gz`;

	buildStore.addLog(buildId, "Creating Rivet actor for kaniko build...");

	try {
		const createResponse = await client.actors.create({
			project: projectId,
			environment: environmentName,
			body: {
				tags: {
					name: "ci-runner",
				},
				build: kanikoBuildId,
				runtime: {
					environment: {
						KANIKO_ARGS: serializeKanikoArguments({
							contextUrl,
							outputUrl,
							destination: `${buildId}:latest`,
							dockerfilePath: build.dockerfilePath,
							buildArgs: build.buildArgs,
							buildTarget: build.buildTarget,
						})
					},
				},
				network: {
					ports: {},
					waitReady: false,
				},
				resources: {
					cpu: 1000,
					memory: 1024,
				},
				lifecycle: {
					killTimeout: 300000,
					durable: false,
				},
			},
		});

		const actorId = createResponse.actor.id;
		buildStore.addLog(buildId, `Created Rivet actor: ${actorId}`);

		buildStore.updateStatus(buildId, {
			type: "running",
			data: {
				rivet: { actorId }
			}
		});

		await pollActorStatus(
			buildStore,
			client,
			projectId,
			environmentName,
			buildId,
			actorId,
		);
	} catch (error: any) {
		buildStore.addLog(
			buildId,
			`Failed to create Rivet actor: ${error.message}`,
		);
		buildStore.updateStatus(buildId, {
			type: "failure",
			data: { reason: `Failed to create Rivet actor: ${error.message}` },
		});
	}
}

async function pollActorStatus(
	buildStore: BuildStore,
	client: RivetClient,
	projectId: string,
	environmentName: string,
	buildId: string,
	actorId: string,
): Promise<void> {
	const pollInterval = 2000;
	const maxPolls = 300;
	let pollCount = 0;

	while (true) {
		try {
			pollCount++;
			if (pollCount > maxPolls) {
				buildStore.addLog(buildId, "Polling timeout reached");
				buildStore.updateStatus(buildId, {
					type: "failure",
					data: { reason: "Actor polling timeout" },
				});
				return;
			}

			const { actor } = await client.actors.get(actorId, {
				project: projectId,
				environment: environmentName,
			});

			let state: string;
			if (actor.destroyedAt && actor.startedAt) {
				if (actor.startedAt) {
					state = "stopped";
				} else {
					state = "crashed";
				}
			} else if (actor.startedAt) {
				state = "running";
			} else {
				state = "starting";
			}

			buildStore.addLog(buildId, `Actor status: ${state}`);

			if (state === "stopped") {
				buildStore.addLog(buildId, `Actor stopped.`);
				return;
			}

			if (state === "crashed") {
				buildStore.updateStatus(buildId, {
					type: "failure",
					data: { reason: `Actor crashed` },
				});
				return;
			}

			if (state === "running" || state === "starting") {
				await new Promise((resolve) => setTimeout(resolve, 1000));
			} else {
				buildStore.addLog(buildId, `Unexpected actor state: ${state}`);
				buildStore.updateStatus(buildId, {
					type: "failure",
					data: { reason: `Unexpected actor state: ${state}` },
				});
				return;
			}
		} catch (error: any) {
			buildStore.addLog(buildId, `Error polling actor: ${error.message}`);
			buildStore.updateStatus(buildId, {
				type: "failure",
				data: { reason: `Error polling actor: ${error.message}` },
			});
			return;
		}
	}
}
