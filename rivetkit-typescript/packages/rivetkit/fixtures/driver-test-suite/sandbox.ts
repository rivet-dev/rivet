import { request as httpRequest } from "node:http";
import { actor } from "rivetkit";
import { sandboxActor, type SandboxProvider } from "rivetkit/sandbox";

const SANDBOX_AGENT_IMAGE = "rivetdev/sandbox-agent:0.5.0-rc.2-full";
const DOCKER_SOCKET_PATH = "/var/run/docker.sock";
const SANDBOX_AGENT_PORT = 3000;
const DOCKER_SANDBOX_CONTROL_KEY = ["docker-sandbox-control"];
let sandboxImageReady: Promise<void> | undefined;

interface DockerResponse {
	statusCode: number;
	body: string;
}

function dockerSocketRequest(
	method: string,
	path: string,
	body?: unknown,
): Promise<DockerResponse> {
	return new Promise((resolve, reject) => {
		const payload = body === undefined ? undefined : JSON.stringify(body);
		const req = httpRequest(
			{
				socketPath: DOCKER_SOCKET_PATH,
				path,
				method,
				headers:
					payload === undefined
						? undefined
						: {
								"content-type": "application/json",
								"content-length": Buffer.byteLength(payload),
							},
			},
			(res) => {
				const chunks: Buffer[] = [];
				res.on("data", (chunk) => {
					chunks.push(
						Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk),
					);
				});
				res.on("end", () => {
					resolve({
						statusCode: res.statusCode ?? 0,
						body: Buffer.concat(chunks).toString("utf8"),
					});
				});
				res.on("error", reject);
			},
		);
		req.on("error", reject);
		if (payload !== undefined) {
			req.write(payload);
		}
		req.end();
	});
}

function assertDockerSuccess(
	response: DockerResponse,
	context: string,
	allowedStatusCodes: number[] = [],
): void {
	if (
		(response.statusCode >= 200 && response.statusCode < 300) ||
		allowedStatusCodes.includes(response.statusCode)
	) {
		return;
	}

	throw new Error(
		`${context} failed with status ${response.statusCode}: ${response.body}`,
	);
}

async function ensureSandboxImage(): Promise<void> {
	if (sandboxImageReady) {
		await sandboxImageReady;
		return;
	}

	sandboxImageReady = (async () => {
		const inspectImage = await dockerSocketRequest(
			"GET",
			`/images/${encodeURIComponent(SANDBOX_AGENT_IMAGE)}/json`,
		);
		if (inspectImage.statusCode === 404) {
			const pullImage = await dockerSocketRequest(
				"POST",
				`/images/create?fromImage=${encodeURIComponent(SANDBOX_AGENT_IMAGE)}`,
			);
			assertDockerSuccess(pullImage, "docker image pull");
			return;
		}
		assertDockerSuccess(inspectImage, "docker image inspect");
	})();

	try {
		await sandboxImageReady;
	} catch (error) {
		sandboxImageReady = undefined;
		throw error;
	}
}

function extractMappedPort(containerInfo: {
	NetworkSettings?: {
		Ports?: Record<
			string,
			Array<{
				HostPort?: string;
			}> | null
		>;
	};
}): number {
	const hostPort =
		containerInfo.NetworkSettings?.Ports?.[`${SANDBOX_AGENT_PORT}/tcp`]?.[0]
			?.HostPort;
	if (!hostPort) {
		throw new Error(
			`docker sandbox-agent port ${SANDBOX_AGENT_PORT} is not published`,
		);
	}
	return Number(hostPort);
}

async function inspectContainer(sandboxId: string): Promise<{
	NetworkSettings?: {
		Ports?: Record<
			string,
			Array<{
				HostPort?: string;
			}> | null
		>;
	};
}> {
	const containerId = normalizeSandboxId(sandboxId);
	const response = await dockerSocketRequest(
		"GET",
		`/containers/${containerId}/json`,
	);
	assertDockerSuccess(response, "docker container inspect");
	return JSON.parse(response.body) as {
		NetworkSettings?: {
			Ports?: Record<
				string,
				Array<{
					HostPort?: string;
				}> | null
			>;
		};
	};
}

function normalizeSandboxId(sandboxId: string): string {
	return sandboxId.startsWith("docker/")
		? sandboxId.slice("docker/".length)
		: sandboxId;
}

export const dockerSandboxControlActor = actor({
	options: {
		actionTimeout: 120_000,
	},
	actions: {
		ensureSandboxImage: async () => {
			await ensureSandboxImage();
		},
		createSandboxContainer: async () => {
			await ensureSandboxImage();
			const createContainer = await dockerSocketRequest(
				"POST",
				"/containers/create",
				{
					Image: SANDBOX_AGENT_IMAGE,
					Cmd: [
						"server",
						"--no-token",
						"--host",
						"0.0.0.0",
						"--port",
						String(SANDBOX_AGENT_PORT),
					],
					ExposedPorts: {
						[`${SANDBOX_AGENT_PORT}/tcp`]: {},
					},
					HostConfig: {
						AutoRemove: true,
						PublishAllPorts: true,
					},
				},
			);
			assertDockerSuccess(createContainer, "docker container create");
			const container = JSON.parse(createContainer.body) as {
				Id?: string;
			};
			if (!container.Id) {
				throw new Error(
					`docker container create returned no id: ${createContainer.body}`,
				);
			}
			const startContainer = await dockerSocketRequest(
				"POST",
				`/containers/${container.Id}/start`,
			);
			assertDockerSuccess(startContainer, "docker container start");
			return container.Id;
		},
		destroySandboxContainer: async (_c, sandboxId: string) => {
			const containerId = normalizeSandboxId(sandboxId);
			const stopContainer = await dockerSocketRequest(
				"POST",
				`/containers/${containerId}/stop?t=5`,
			);
			assertDockerSuccess(
				stopContainer,
				"docker container stop",
				[304, 404],
			);
			const deleteContainer = await dockerSocketRequest(
				"DELETE",
				`/containers/${containerId}?force=true`,
			);
			assertDockerSuccess(
				deleteContainer,
				"docker container delete",
				[404],
			);
		},
		getSandboxUrl: async (_c, sandboxId: string) => {
			const containerInfo = await inspectContainer(sandboxId);
			const hostPort = extractMappedPort(containerInfo);
			return `http://127.0.0.1:${hostPort}`;
		},
	},
});

export const dockerSandboxActor = sandboxActor({
	createProvider: (c) => {
		const controller = c
			.client<any>()
			.dockerSandboxControlActor.getOrCreate(DOCKER_SANDBOX_CONTROL_KEY);

		const provider: SandboxProvider = {
			name: "docker",
			defaultCwd: "/home/sandbox",
			create: async () => {
				return await controller.createSandboxContainer();
			},
			destroy: async (sandboxId) => {
				await controller.destroySandboxContainer(sandboxId);
			},
			getUrl: async (sandboxId) => {
				return await controller.getSandboxUrl(sandboxId);
			},
		};

		return provider;
	},
});
