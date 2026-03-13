import { SandboxAgent } from "sandbox-agent";
import type {
	SandboxActorProvider,
	SandboxActorProviderConnectOptions,
	SandboxActorProviderCreateContext,
} from "../types";
import { importOptionalDependency } from "./shared";

const DEFAULT_AGENT_PORT = 3000;
const DEFAULT_INSTALL_COMMAND =
	"curl -fsSL https://releases.rivet.dev/sandbox-agent/0.3.x/install.sh | sh";

type E2BSandboxLike = {
	sandboxId?: string;
	id?: string;
	commands: {
		run(
			command: string,
			options?: Record<string, unknown>,
		): Promise<{ exitCode?: number; stderr?: string }>;
	};
	getHost(port: number): string;
	kill(): Promise<void>;
};

type E2BSandboxStatic = {
	create(options?: Record<string, unknown>): Promise<unknown>;
	connect(
		sandboxId: string,
		options?: Record<string, unknown>,
	): Promise<unknown>;
};

export interface E2BProviderOptions {
	create?:
		| Record<string, unknown>
		| ((
				context: SandboxActorProviderCreateContext,
		  ) => Record<string, unknown> | Promise<Record<string, unknown>>);
	connect?:
		| Record<string, unknown>
		| ((
				sandboxId: string,
		  ) => Record<string, unknown> | Promise<Record<string, unknown>>);
	installAgents?: string[];
	agentPort?: number;
	installCommand?: string;
	startCommand?: string;
}

async function loadE2B(): Promise<E2BSandboxStatic> {
	const module =
		await importOptionalDependency<typeof import("@e2b/code-interpreter")>(
			"@e2b/code-interpreter",
			"e2b",
		);
	return module.Sandbox;
}

async function resolveCreateOptions(
	options: E2BProviderOptions,
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

async function resolveConnectOptions(
	options: E2BProviderOptions,
	sandboxId: string,
): Promise<Record<string, unknown>> {
	if (!options.connect) {
		return {};
	}

	if (typeof options.connect === "function") {
		return await options.connect(sandboxId);
	}

	return options.connect;
}

async function runOrThrow(
	sandbox: E2BSandboxLike,
	command: string,
	options?: Record<string, unknown>,
): Promise<void> {
	const result = await sandbox.commands.run(command, options);
	if (result.exitCode && result.exitCode !== 0) {
		throw new Error(
			`e2b command failed: ${command}\n${result.stderr ?? "unknown error"}`,
		);
	}
}

function buildStartCommand(port: number, explicit?: string): string {
	if (explicit) {
		return explicit;
	}

	return `sandbox-agent server --no-token --host 0.0.0.0 --port ${port}`;
}

export function e2b(options: E2BProviderOptions = {}): SandboxActorProvider {
	const installAgents = options.installAgents ?? ["claude", "codex"];
	const agentPort = options.agentPort ?? DEFAULT_AGENT_PORT;

	return {
		name: "e2b",
		async create(context) {
			const sandboxModule = await loadE2B();
			const createOptions = await resolveCreateOptions(options, context);
			const sandbox = (await sandboxModule.create({
				allowInternetAccess: true,
				...createOptions,
			})) as E2BSandboxLike;

			await runOrThrow(
				sandbox,
				options.installCommand ?? DEFAULT_INSTALL_COMMAND,
			);
			for (const agent of installAgents) {
				await runOrThrow(sandbox, `sandbox-agent install-agent ${agent}`);
			}
			await runOrThrow(
				sandbox,
				buildStartCommand(agentPort, options.startCommand),
				{ background: true, timeoutMs: 0 },
			);

			const sandboxId = sandbox.id ?? sandbox.sandboxId;
			if (!sandboxId) {
				throw new Error("e2b sandbox did not return an id");
			}

			return sandboxId;
		},
		async destroy(sandboxId) {
			const sandboxModule = await loadE2B();
			const connectOptions = await resolveConnectOptions(options, sandboxId);
			const sandbox = (await sandboxModule.connect(
				sandboxId,
				connectOptions,
			)) as E2BSandboxLike;
			await sandbox.kill();
		},
		async connectAgent(
			sandboxId,
			connectOptions: SandboxActorProviderConnectOptions,
		) {
			const sandboxModule = await loadE2B();
			const e2bConnectOptions = await resolveConnectOptions(options, sandboxId);
			const sandbox = (await sandboxModule.connect(
				sandboxId,
				e2bConnectOptions,
			)) as E2BSandboxLike;
			const baseUrl = `https://${sandbox.getHost(agentPort)}`;

			return await SandboxAgent.connect({
				baseUrl,
				...connectOptions,
			});
		},
	};
}
