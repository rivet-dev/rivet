import type { Daytona } from "@daytonaio/sdk";
import { SandboxAgent } from "sandbox-agent";
import type {
	SandboxActorProvider,
	SandboxActorProviderConnectOptions,
	SandboxActorProviderCreateContext,
} from "../types";
import { importOptionalDependency } from "./shared";

const DEFAULT_AGENT_PORT = 3000;
const DEFAULT_PREVIEW_TTL_SECONDS = 4 * 60 * 60;
const DEFAULT_INSTALL_COMMAND =
	"curl -fsSL https://releases.rivet.dev/sandbox-agent/0.3.x/install.sh | sh";

type DaytonaSandboxLike = {
	id: string;
	process: {
		executeCommand(command: string): Promise<unknown>;
	};
	getSignedPreviewUrl(port: number, ttlSeconds: number): Promise<{ url: string }>;
	delete(timeoutSeconds?: number): Promise<void>;
};

type DaytonaClientLike = Pick<Daytona, "create"> & {
	get?(sandboxId: string): Promise<DaytonaSandboxLike | undefined>;
};

export interface DaytonaProviderOptions {
	client?: DaytonaClientLike;
	create?:
		| Record<string, unknown>
		| ((
				context: SandboxActorProviderCreateContext,
		  ) => Record<string, unknown> | Promise<Record<string, unknown>>);
	installAgents?: string[];
	agentPort?: number;
	previewTtlSeconds?: number;
	installCommand?: string;
	startCommand?: string;
	deleteTimeoutSeconds?: number;
}

async function resolveCreateOptions(
	options: DaytonaProviderOptions,
	context: SandboxActorProviderCreateContext,
): Promise<Record<string, unknown>> {
	if (!options.create) {
		return {};
	}

	if (typeof options.create === "function") {
		return await options.create(context);
	}

	return options.create;
}

function buildStartCommand(port: number, explicit?: string): string {
	if (explicit) {
		return explicit;
	}

	return `nohup sandbox-agent server --no-token --host 0.0.0.0 --port ${port} >/tmp/sandbox-agent.log 2>&1 &`;
}

function buildInstallAgentCommands(agents: string[]): string[] {
	return agents.map((agent) => `sandbox-agent install-agent ${agent}`);
}

async function createDefaultClient(): Promise<DaytonaClientLike> {
	const { Daytona } = await importOptionalDependency<typeof import("@daytonaio/sdk")>(
		"@daytonaio/sdk",
		"daytona",
	);
	return new Daytona();
}

export function daytona(
	options: DaytonaProviderOptions = {},
): SandboxActorProvider {
	const installAgents = options.installAgents ?? ["claude", "codex"];
	const agentPort = options.agentPort ?? DEFAULT_AGENT_PORT;
	const previewTtlSeconds =
		options.previewTtlSeconds ?? DEFAULT_PREVIEW_TTL_SECONDS;

	return {
		name: "daytona",
		async create(context) {
			const client = options.client ?? (await createDefaultClient());
			const createOptions = await resolveCreateOptions(options, context);
			const sandbox = (await client.create({
				autoStopInterval: 0,
				...createOptions,
			})) as DaytonaSandboxLike;

			await sandbox.process.executeCommand(
				options.installCommand ?? DEFAULT_INSTALL_COMMAND,
			);
			for (const command of buildInstallAgentCommands(installAgents)) {
				await sandbox.process.executeCommand(command);
			}
			await sandbox.process.executeCommand(
				buildStartCommand(agentPort, options.startCommand),
			);

			return sandbox.id;
		},
		async destroy(sandboxId) {
			const client = options.client ?? (await createDefaultClient());
			const sandbox = await client.get?.(sandboxId);
			if (!sandbox) {
				return;
			}
			await sandbox.delete(options.deleteTimeoutSeconds);
		},
		async connectAgent(
			sandboxId,
			connectOptions: SandboxActorProviderConnectOptions,
		) {
			const client = options.client ?? (await createDefaultClient());
			const sandbox = await client.get?.(sandboxId);
			if (!sandbox) {
				throw new Error(`daytona sandbox not found: ${sandboxId}`);
			}

			const { url } = await sandbox.getSignedPreviewUrl(
				agentPort,
				previewTtlSeconds,
			);

			return await SandboxAgent.connect({
				baseUrl: url,
				...connectOptions,
			});
		},
	};
}
