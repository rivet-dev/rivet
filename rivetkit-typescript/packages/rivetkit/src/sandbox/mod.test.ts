import { afterEach, beforeAll, describe, expect, test, vi } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { DatabaseProvider } from "@/actor/database";
import type { RawAccess } from "@/db/config";
import type { SandboxAgent } from "sandbox-agent";
import type { SandboxActorProvider } from "./types";

let sandboxModule: typeof import("./mod");

const databaseStub = {
	createClient: async () => ({
		execute: async () => [],
		close: async () => {},
	}),
	onMigrate: async () => {},
} satisfies DatabaseProvider<RawAccess>;

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
		.filter((name): name is string => name !== null && name !== "constructor")
		.sort();
}

function createContext() {
	return {
		actorId: "actor-1",
		key: ["sandbox"] as const,
		state: {
			sandboxId: null as string | null,
			sessionIds: [] as string[],
			providerName: "stub" as string | null,
			activeSessionIds: [] as string[],
			activePromptRequestIdsBySessionId: {} as Record<string, string[]>,
			activeSessionLastEventAtById: {} as Record<string, number>,
		},
		vars: {
			agent: null as SandboxAgent | null,
			provider: null as SandboxActorProvider | null,
			unsubscribeBySessionId: new Map(),
			activeHookCount: 0,
			warningTimeoutBySessionId: new Map(),
			staleTimeoutBySessionId: new Map(),
		},
		db: {
			execute: vi.fn(async () => []),
			close: vi.fn(async () => {}),
		},
		waitUntil: vi.fn((promise: Promise<void>) => {
			void promise;
		}),
		setPreventSleep: vi.fn(),
		log: {
			error: vi.fn(),
			warn: vi.fn(),
		},
	};
}

describe("sandbox actor sdk parity", () => {
	beforeAll(async () => {
		sandboxModule = await import("./mod");
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	test("keeps the hook and action split in sync with sandbox-agent", () => {
		const expected = getPublicSandboxAgentSdkMethods();
		const actual = [
			...sandboxModule.SANDBOX_AGENT_HOOK_METHODS,
			...sandboxModule.SANDBOX_AGENT_ACTION_METHODS,
		].sort();

		expect(actual).toEqual(expected);
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
		const definition = sandboxModule.sandboxActor({
			provider: providerStub,
			database: databaseStub,
		});
		const actionNames = Object.keys(definition.config.actions ?? {}).sort();

		expect(actionNames).toEqual([
			...sandboxModule.SANDBOX_AGENT_ACTION_METHODS,
		].sort());
	});

	test("re-subscribes persisted sessions after reconnecting the agent", async () => {
		const subscriptionCalls: string[] = [];

		function createAgent(label: string): SandboxAgent {
			return {
				dispose: vi.fn(async () => {}),
				getSession: vi.fn(async (id: string) => ({ id })),
				listAgents: vi.fn(async () => ({ agents: [] })),
				onSessionEvent: vi.fn((sessionId: string) => {
					subscriptionCalls.push(`${label}:event:${sessionId}`);
					return () => {};
				}),
				onPermissionRequest: vi.fn((sessionId: string) => {
					subscriptionCalls.push(`${label}:permission:${sessionId}`);
					return () => {};
				}),
			} as unknown as SandboxAgent;
		}

		const firstAgent = createAgent("first");
		const secondAgent = createAgent("second");
		const providerStub: SandboxActorProvider = {
			name: "stub",
			create: vi.fn(async () => "sandbox-1"),
			destroy: vi.fn(async () => {}),
			connectAgent: vi
				.fn<() => Promise<SandboxAgent>>()
				.mockResolvedValueOnce(firstAgent)
				.mockResolvedValueOnce(secondAgent),
		};
		const definition = sandboxModule.sandboxActor({
			provider: providerStub,
			database: databaseStub,
		});
		const actions = definition.config.actions!;
		const context = createContext();

		await actions.getSession(context as never, "session-1");
		await actions.dispose(context as never);
		await actions.listAgents(context as never);

		expect(subscriptionCalls).toEqual([
			"first:event:session-1",
			"first:permission:session-1",
			"second:event:session-1",
			"second:permission:session-1",
		]);
	});

	test("preventSleep tracks active prompt turns from session events", async () => {
		vi.useFakeTimers();

		let sessionEventListener:
			| ((event: Parameters<SandboxAgent["onSessionEvent"]>[1] extends (
					event: infer T,
				) => void
					? T
					: never) => void)
			| undefined;

		const agent = {
			dispose: vi.fn(async () => {}),
			getSession: vi.fn(async (id: string) => ({ id })),
			onSessionEvent: vi.fn(
				(
					_sessionId: string,
					listener: Parameters<SandboxAgent["onSessionEvent"]>[1],
				) => {
					sessionEventListener = listener;
					return () => {};
				},
			),
			onPermissionRequest: vi.fn(() => () => {}),
		} as unknown as SandboxAgent;

		const providerStub: SandboxActorProvider = {
			name: "stub",
			create: vi.fn(async () => "sandbox-1"),
			destroy: vi.fn(async () => {}),
			connectAgent: vi.fn(async () => agent),
		};
		const definition = sandboxModule.sandboxActor({
			provider: providerStub,
			database: databaseStub,
			preventSleepWhileTurnsActive: true,
		});
		const context = createContext();

		await definition.config.actions!.getSession(context as never, "session-1");

		sessionEventListener?.({
			id: "event-1",
			eventIndex: 0,
			sessionId: "session-1",
			createdAt: Date.now(),
			connectionId: "conn-1",
			sender: "client",
			payload: {
				jsonrpc: "2.0",
				id: "prompt-1",
				method: "session/prompt",
				params: {
					sessionId: "session-1",
				},
			},
		});

		expect(context.state.activeSessionIds).toEqual(["session-1"]);
		expect(
			context.state.activePromptRequestIdsBySessionId["session-1"],
		).toEqual(["prompt-1"]);
		expect(context.setPreventSleep).toHaveBeenLastCalledWith(true);

		sessionEventListener?.({
			id: "event-2",
			eventIndex: 1,
			sessionId: "session-1",
			createdAt: Date.now(),
			connectionId: "conn-1",
			sender: "agent",
			payload: {
				jsonrpc: "2.0",
				id: "prompt-1",
				result: {
					stopReason: "end_turn",
				},
			},
		});

		expect(context.state.activeSessionIds).toEqual([]);
		expect(context.setPreventSleep).toHaveBeenLastCalledWith(false);
	});

	test("preventSleep is restored on wake when a turn is still active", async () => {
		vi.useFakeTimers();

		const subscriptionCalls: string[] = [];
		const providerStub: SandboxActorProvider = {
			name: "stub",
			create: vi.fn(async () => "sandbox-1"),
			destroy: vi.fn(async () => {}),
			connectAgent: vi.fn(async () => {
				return {
					dispose: vi.fn(async () => {}),
					onSessionEvent: vi.fn((sessionId: string) => {
						subscriptionCalls.push(`event:${sessionId}`);
						return () => {};
					}),
					onPermissionRequest: vi.fn((sessionId: string) => {
						subscriptionCalls.push(`permission:${sessionId}`);
						return () => {};
					}),
				} as unknown as SandboxAgent;
			}),
		};
		const definition = sandboxModule.sandboxActor({
			provider: providerStub,
			database: databaseStub,
			preventSleepWhileTurnsActive: true,
		});
		const context = createContext();
		context.state.sandboxId = "sandbox-1";
		context.state.sessionIds = ["session-1"];
		context.state.activeSessionIds = ["session-1"];
		context.state.activePromptRequestIdsBySessionId = {
			"session-1": ["prompt-1"],
		};
		context.state.activeSessionLastEventAtById = {
			"session-1": Date.now(),
		};

		await definition.config.onWake?.(context as never);

		expect(context.setPreventSleep).toHaveBeenLastCalledWith(true);
		expect(subscriptionCalls).toEqual([
			"event:session-1",
			"permission:session-1",
		]);
	});
});
