import type Docker from "dockerode";
import getPort from "get-port";
import { SandboxAgent } from "sandbox-agent";
import type {
	SandboxActorProvider,
	SandboxActorProviderConnectOptions,
	SandboxActorProviderCreateContext,
} from "../types";
import { importOptionalDependency } from "./shared";

const DEFAULT_IMAGE = "node:22-bookworm-slim";
const DEFAULT_HOST = "127.0.0.1";
const DEFAULT_AGENT_PORT = 3000;
const DEFAULT_INSTALL_COMMAND =
	"curl -fsSL https://releases.rivet.dev/sandbox-agent/0.3.x/install.sh | sh";

type DockerClientLike = Pick<Docker, "createContainer" | "getContainer">;
type DockerConstructor = new (options: {
	socketPath: string;
}) => DockerClientLike;

export interface DockerProviderOptions {
	client?: DockerClientLike;
	image?: string;
	host?: string;
	agentPort?: number;
	env?:
		| string[]
		| ((
				context: SandboxActorProviderCreateContext,
		  ) => string[] | Promise<string[]>);
	binds?:
		| string[]
		| ((
				context: SandboxActorProviderCreateContext,
		  ) => string[] | Promise<string[]>);
	installAgents?: string[];
	installCommand?: string;
	startCommand?: string;
	createContainerOptions?: Record<string, unknown>;
}

async function resolveValue<T>(
	value: T | ((context: SandboxActorProviderCreateContext) => T | Promise<T>) | undefined,
	context: SandboxActorProviderCreateContext,
	fallback: T,
): Promise<T> {
	if (value === undefined) {
		return fallback;
	}

	if (typeof value === "function") {
		return await (
			value as (context: SandboxActorProviderCreateContext) => T | Promise<T>
		)(context);
	}

	return value;
}

function buildContainerCommand(
	options: DockerProviderOptions,
	agentPort: number,
	installAgents: string[],
): string {
	const startCommand =
		options.startCommand ??
		`sandbox-agent server --no-token --host 0.0.0.0 --port ${agentPort}`;
	return [
		"apt-get update",
		"DEBIAN_FRONTEND=noninteractive apt-get install -y curl ca-certificates bash libstdc++6",
		"rm -rf /var/lib/apt/lists/*",
		options.installCommand ?? DEFAULT_INSTALL_COMMAND,
		...installAgents.map((agent) => `sandbox-agent install-agent ${agent}`),
		startCommand,
	].join(" && ");
}

function extractMappedPort(containerInfo: any, containerPort: number): number {
	const ports = containerInfo?.NetworkSettings?.Ports;
	const binding = ports?.[`${containerPort}/tcp`];
	const hostPort = binding?.[0]?.HostPort;
	if (!hostPort) {
		throw new Error(`docker sandbox-agent port ${containerPort} is not published`);
	}

	return Number(hostPort);
}

async function createDefaultClient(): Promise<DockerClientLike> {
	const module = await importOptionalDependency<typeof import("dockerode")>(
		"dockerode",
		"docker",
	);
	const DockerCtor =
		(module as unknown as { default?: DockerConstructor }).default ??
		(module as unknown as DockerConstructor);
	return new DockerCtor({ socketPath: "/var/run/docker.sock" });
}

export function docker(
	options: DockerProviderOptions = {},
): SandboxActorProvider {
	const image = options.image ?? DEFAULT_IMAGE;
	const host = options.host ?? DEFAULT_HOST;
	const agentPort = options.agentPort ?? DEFAULT_AGENT_PORT;
	const installAgents = options.installAgents ?? ["claude", "codex"];

	return {
		name: "docker",
		async create(context) {
			const client = options.client ?? (await createDefaultClient());
			const env = await resolveValue(options.env, context, []);
			const binds = await resolveValue(options.binds, context, []);
			const hostPort = await getPort();
			const container = await client.createContainer({
				Image: image,
				Cmd: ["sh", "-c", buildContainerCommand(options, agentPort, installAgents)],
				Env: env,
				ExposedPorts: { [`${agentPort}/tcp`]: {} },
				HostConfig: {
					AutoRemove: true,
					Binds: binds,
					PortBindings: {
						[`${agentPort}/tcp`]: [{ HostPort: String(hostPort) }],
					},
				},
				...(options.createContainerOptions ?? {}),
			} as any);
			await container.start();
			return container.id;
		},
		async destroy(sandboxId) {
			const client = options.client ?? (await createDefaultClient());
			const container = client.getContainer(sandboxId);
			try {
				await container.stop({ t: 5 });
			} catch {}
			try {
				await container.remove({ force: true });
			} catch {}
		},
		async connectAgent(
			sandboxId,
			connectOptions: SandboxActorProviderConnectOptions,
		) {
			const client = options.client ?? (await createDefaultClient());
			const container = client.getContainer(sandboxId);
			const info = await container.inspect();
			const hostPort = extractMappedPort(info, agentPort);
			return await SandboxAgent.connect({
				baseUrl: `http://${host}:${hostPort}`,
				...connectOptions,
			});
		},
	};
}
