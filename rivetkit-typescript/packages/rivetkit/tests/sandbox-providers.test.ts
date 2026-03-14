import { afterEach, describe, expect, test, vi } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { sandboxActor } from "../src/sandbox/index";
import {
	SANDBOX_AGENT_ACTION_METHODS,
	SANDBOX_AGENT_HOOK_METHODS,
	type SandboxActorProvider,
} from "../src/sandbox/types";

// --- SDK parity tests ---

function getPublicSandboxAgentSdkMethods(): string[] {
	let dir = path.dirname(fileURLToPath(import.meta.url));
	let declarationsPath: string | null = null;

	while (dir !== path.dirname(dir)) {
		const candidate = path.join(
			dir,
			"node_modules/sandbox-agent/dist/index.d.ts",
		);
		if (fs.existsSync(candidate)) {
			declarationsPath = candidate;
			break;
		}
		dir = path.dirname(dir);
	}

	if (!declarationsPath) {
		throw new Error("unable to locate sandbox-agent declarations");
	}

	const declarations = fs.readFileSync(declarationsPath, "utf8");
	const match = declarations.match(
		/declare class SandboxAgent \{([\s\S]*?)^\}/m,
	);
	if (!match) {
		throw new Error("unable to locate SandboxAgent declaration block");
	}

	return match[1]
		.split("\n")
		.map((line) => line.trim())
		.filter((line) => line.length > 0)
		.filter((line) => !line.startsWith("private "))
		.filter((line) => !line.startsWith("static "))
		.map((line) => line.match(/^([A-Za-z0-9_]+)\(/)?.[1] ?? null)
		.filter(
			(name): name is string => name !== null && name !== "constructor",
		)
		.sort();
}

describe("sandbox actor sdk parity", () => {
	test("keeps the hook and action split in sync with sandbox-agent", () => {
		expect(
			[
				...SANDBOX_AGENT_HOOK_METHODS,
				...SANDBOX_AGENT_ACTION_METHODS,
			].sort(),
		).toEqual(getPublicSandboxAgentSdkMethods());
	});

	test("exposes every sandbox-agent action method on the actor definition", () => {
		const providerStub: SandboxActorProvider = {
			name: "stub",
			async create() {
				throw new Error("not implemented");
			},
			async destroy() {
				throw new Error("not implemented");
			},
			async connectAgent() {
				throw new Error("not implemented");
			},
		};
		const definition = sandboxActor({
			provider: providerStub,
		});

		const actionKeys = Object.keys(definition.config.actions ?? {}).sort();
		// The sandbox actor adds a custom `destroy` action alongside all
		// proxied sandbox-agent methods.
		expect(actionKeys).toEqual(
			[...SANDBOX_AGENT_ACTION_METHODS, "destroy"].sort(),
		);
	});
});

// --- Generic provider lifecycle test suite ---

interface ProviderFixture {
	name: string;
	setup: () => Promise<{
		provider: SandboxActorProvider;
		expectedSandboxId: string;
		expectedBaseUrl: string;
		verifyCreate: () => void;
		verifyConnect: () => void;
		verifyDestroy: () => void;
	}>;
}

const CREATE_CONTEXT = { actorId: "actor-1", actorKey: ["sandbox"] };
const CONNECT_OPTIONS = { persist: {} as never, waitForHealth: true };

function dockerFixture(): ProviderFixture {
	return {
		name: "docker",
		async setup() {
			const { docker } = await import(
				"../src/sandbox/providers/docker"
			);

			const start = vi.fn(async () => {});
			const stop = vi.fn(async () => {});
			const remove = vi.fn(async () => {});
			const inspect = vi.fn(async () => ({
				NetworkSettings: {
					Ports: {
						"3000/tcp": [{ HostPort: "42123" }],
					},
				},
			}));
			const createContainer = vi.fn(async (config: unknown) => ({
				id: "container-1",
				start,
				inspect,
				stop,
				remove,
				config,
			}));
			const getContainer = vi.fn(() => ({
				inspect,
				stop,
				remove,
			}));

			const provider = docker({
				client: { createContainer, getContainer },
				image: "example-image",
				env: async () => ["FOO=bar"],
				binds: async () => ["/tmp:/tmp"],
				installAgents: ["codex"],
				installCommand: "install-cmd",
				startCommand: "start-cmd",
			});

			return {
				provider,
				expectedSandboxId: "container-1",
				expectedBaseUrl: "http://127.0.0.1:42123",
				verifyCreate() {
					expect(start).toHaveBeenCalledOnce();
					expect(createContainer).toHaveBeenCalledWith(
						expect.objectContaining({
							Image: "example-image",
							Env: ["FOO=bar"],
							Cmd: [
								"sh",
								"-c",
								expect.stringContaining("install-cmd"),
							],
							HostConfig: expect.objectContaining({
								Binds: ["/tmp:/tmp"],
							}),
						}),
					);
					const cmd = (
						createContainer.mock.calls[0]?.[0] as {
							Cmd?: string[];
						}
					).Cmd?.[2];
					expect(cmd).toContain(
						"sandbox-agent install-agent codex",
					);
					expect(cmd).toContain("start-cmd");
				},
				verifyConnect() {
					expect(getContainer).toHaveBeenCalledWith("container-1");
				},
				verifyDestroy() {
					expect(stop).toHaveBeenCalledWith({ t: 5 });
					expect(remove).toHaveBeenCalledWith({ force: true });
				},
			};
		},
	};
}

function daytonaFixture(): ProviderFixture {
	return {
		name: "daytona",
		async setup() {
			const { daytona } = await import(
				"../src/sandbox/providers/daytona"
			);

			const executeCommand = vi.fn(async () => ({}));
			const deleteSandbox = vi.fn(async () => {});
			const getSignedPreviewUrl = vi.fn(async () => ({
				url: "https://sandbox-preview.example",
			}));
			const sandbox = {
				id: "sandbox-1",
				process: { executeCommand },
				getSignedPreviewUrl,
				delete: deleteSandbox,
			};
			const createSandbox = vi.fn(async () => sandbox);
			const get = vi.fn(async () => sandbox);

			const provider = daytona({
				client: { create: createSandbox, get },
				create: async () => ({ image: "node:22" }),
				installAgents: ["codex"],
				installCommand: "install-cmd",
				startCommand: "start-cmd",
				agentPort: 4321,
				previewTtlSeconds: 321,
				deleteTimeoutSeconds: 9,
			});

			return {
				provider,
				expectedSandboxId: "sandbox-1",
				expectedBaseUrl: "https://sandbox-preview.example",
				verifyCreate() {
					expect(createSandbox).toHaveBeenCalledWith(
						expect.objectContaining({
							autoStopInterval: 0,
							image: "node:22",
						}),
					);
					expect(
						executeCommand.mock.calls.map(
							([command]) => command,
						),
					).toEqual([
						"install-cmd",
						"sandbox-agent install-agent codex",
						"start-cmd",
					]);
				},
				verifyConnect() {
					expect(getSignedPreviewUrl).toHaveBeenCalledWith(
						4321,
						321,
					);
				},
				verifyDestroy() {
					expect(get).toHaveBeenCalledWith("sandbox-1");
					expect(deleteSandbox).toHaveBeenCalledWith(9);
				},
			};
		},
	};
}

function e2bFixture(): ProviderFixture {
	return {
		name: "e2b",
		async setup() {
			const run = vi.fn(async () => ({ exitCode: 0 }));
			const kill = vi.fn(async () => {});
			const createSandbox = { id: "sandbox-1", commands: { run } };
			const connectSandbox = {
				id: "sandbox-1",
				commands: { run },
				getHost: vi.fn(() => "sandbox-host.example"),
				kill,
			};
			const create = vi.fn(async () => createSandbox);
			const connect = vi.fn(async () => connectSandbox);
			const sandboxAgentConnect = vi.fn(
				async (opts: { baseUrl: string }) => ({
					baseUrl: opts.baseUrl,
				}),
			);

			vi.doMock("sandbox-agent", () => ({
				SandboxAgent: { connect: sandboxAgentConnect },
			}));
			vi.doMock("../src/sandbox/providers/shared", () => ({
				importOptionalDependency: vi.fn(
					async (packageName: string) => {
						expect(packageName).toBe(
							"@e2b/code-interpreter",
						);
						return { Sandbox: { create, connect } };
					},
				),
			}));

			const { e2b } = await import("../src/sandbox/providers/e2b");
			const provider = e2b({
				create: async () => ({ template: "base" }),
				connect: async (sandboxId) => ({ sandboxId }),
				installAgents: ["codex"],
				installCommand: "install-cmd",
				startCommand: "start-cmd",
				agentPort: 4545,
			});

			return {
				provider,
				expectedSandboxId: "sandbox-1",
				expectedBaseUrl: "https://sandbox-host.example",
				verifyCreate() {
					expect(create).toHaveBeenCalledWith(
						expect.objectContaining({
							allowInternetAccess: true,
							template: "base",
						}),
					);
					expect(run.mock.calls).toEqual([
						["install-cmd", undefined],
						[
							"sandbox-agent install-agent codex",
							undefined,
						],
						[
							"start-cmd",
							{ background: true, timeoutMs: 0 },
						],
					]);
				},
				verifyConnect() {
					expect(connect).toHaveBeenCalledWith("sandbox-1", {
						sandboxId: "sandbox-1",
					});
					expect(sandboxAgentConnect).toHaveBeenCalledWith(
						expect.objectContaining({
							baseUrl: "https://sandbox-host.example",
							waitForHealth: true,
						}),
					);
				},
				verifyDestroy() {
					expect(kill).toHaveBeenCalledOnce();
				},
			};
		},
	};
}

// --- Run the generic suite for each provider ---

const FIXTURES = [dockerFixture(), daytonaFixture(), e2bFixture()];

afterEach(() => {
	vi.restoreAllMocks();
	vi.resetModules();
	vi.doUnmock("sandbox-agent");
	vi.doUnmock("../src/sandbox/providers/shared");
});

for (const fixture of FIXTURES) {
	describe(`${fixture.name} provider`, () => {
		test("create, connect, and destroy lifecycle", async () => {
			const harness = await fixture.setup();

			expect(harness.provider.name).toBe(fixture.name);

			const sandboxId = await harness.provider.create(CREATE_CONTEXT);
			expect(sandboxId).toBe(harness.expectedSandboxId);
			harness.verifyCreate();

			const agent = await harness.provider.connectAgent(
				sandboxId,
				CONNECT_OPTIONS,
			);
			expect(agent).toBeDefined();
			expect(
				(agent as { baseUrl?: string }).baseUrl,
			).toBe(harness.expectedBaseUrl);
			harness.verifyConnect();

			await harness.provider.destroy(sandboxId);
			harness.verifyDestroy();
		});
	});
}
