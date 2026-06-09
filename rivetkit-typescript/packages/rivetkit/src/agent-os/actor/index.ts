/**
 * Rust-backed `agentOs(...)` definition (Phase 1c+).
 *
 * Produces an `ActorDefinition` whose `nativeFactoryBuilder` constructs a
 * `CoreActorFactory` through `runtime.createAgentOsFactory(...)` (NAPI →
 * `rivetkit_agent_os::build_core_factory`). All lifecycle, state, and
 * action dispatch live in the Rust crate. The JS shim only validates
 * configuration and hands it across the bridge.
 */

import type {
	BatchReadResult,
	BatchWriteResult,
	CreateSessionOptions,
	CronJobInfo,
	DirEntry,
	JsonRpcResponse,
	ProcessInfo,
	ProcessTreeNode,
	SessionInfo,
	SpawnedProcessInfo,
	VirtualStat,
} from "@rivet-dev/agent-os-core";
import { actor, type ActorDefinition, event } from "@/actor/mod";
import type { ActionContext } from "@/actor/config";
import type { DatabaseProvider, RawAccess } from "@/common/database/config";
import type {
	ActorFactoryHandle,
	CoreRuntime,
	NapiAgentOsOptions,
} from "@/registry/runtime";
import {
	type AgentOsActorConfig,
	type AgentOsActorConfigInput,
	agentOsActorConfigSchema,
} from "../config";
import type {
	AgentOsActorState,
	AgentOsActorVars,
	CronEventPayload,
	PermissionRequestPayload,
	ProcessExitPayload,
	ProcessOutputPayload,
	SessionEventPayload,
	ShellDataPayload,
	VmBootedPayload,
	VmShutdownPayload,
} from "../types";

// Event tokens — declare the payload shape for each event the Rust run
// loop broadcasts. Tokens are passed to the underlying `actor({...})`
// helper as the schema map AND used as the `typeof`-source for the
// declared event surface on `AgentOsActorDefinition`. So a TS consumer
// writing `agent.on("sessionEvent", (data) => ...)` gets `data` typed
// as `SessionEventPayload`.
//
// Only `vmBooted` / `vmShutdown` / `sessionEvent` are currently
// broadcast by the Rust side; the others are declared so the surface
// is stable when the corresponding fan-out wiring lands in later
// phases (process output/exit, shell data, cron events, permission
// requests).
const sessionEventToken = event<SessionEventPayload>();
const permissionRequestToken = event<PermissionRequestPayload>();
const vmBootedToken = event<VmBootedPayload>();
const vmShutdownToken = event<VmShutdownPayload>();
const processOutputToken = event<ProcessOutputPayload>();
const processExitToken = event<ProcessExitPayload>();
const shellDataToken = event<ShellDataPayload>();
const cronEventToken = event<CronEventPayload>();

type AgentOsActorEvents = {
	sessionEvent: typeof sessionEventToken;
	permissionRequest: typeof permissionRequestToken;
	vmBooted: typeof vmBootedToken;
	vmShutdown: typeof vmShutdownToken;
	processOutput: typeof processOutputToken;
	processExit: typeof processExitToken;
	shellData: typeof shellDataToken;
	cronEvent: typeof cronEventToken;
};

/**
 * Build the JSON envelope the Rust crate consumes. The Rust deserializer
 * uses `deny_unknown_fields` + `camelCase`, so this output must stay in
 * lock-step with `packages/rivetkit-napi/src/agent_os.rs::AgentOsConfigJson`.
 *
 * `kind` is inferred from the descriptor shape rather than required from
 * the user. The JS `SoftwareInput` union encodes the discriminator
 * structurally:
 *   - `commandDir: string` (typed `type: "wasm-commands"` or duck-typed
 *     registry packages like `@rivet-dev/agent-os-common`) → wasm
 *     command directory, mounted at `/__agentos/commands/{N}/` by the
 *     Rust client.
 *   - `type: "agent"` + `packageDir: string` → agent SDK package.
 *   - `type: "tool"` + `packageDir: string` → tool package.
 *
 * Meta-packages (`software: [common]` where `common` is itself an
 * array) are shallow-flattened. Malformed descriptors are silently
 * dropped rather than failing the whole config — same fail-soft
 * behavior as the legacy JS port's `processSoftware`.
 */
export function buildConfigJson<TConnParams>(
	parsed: AgentOsActorConfig<TConnParams>,
): string {
	const out: AgentOsConfigJsonEnvelope = {};

	const options = parsed.options as AgentOsOptionsLoose | undefined;

	const rawSoftware = options?.software;
	if (Array.isArray(rawSoftware) && rawSoftware.length > 0) {
		const flat: AgentOsConfigJsonSoftwareEntry[] = [];
		for (const entry of rawSoftware) {
			if (Array.isArray(entry)) {
				for (const descriptor of entry) {
					const mapped = mapSoftwareDescriptor(descriptor);
					if (mapped) flat.push(mapped);
				}
			} else {
				const mapped = mapSoftwareDescriptor(entry);
				if (mapped) flat.push(mapped);
			}
		}
		if (flat.length > 0) out.software = flat;
	}

	if (typeof options?.additionalInstructions === "string") {
		out.additionalInstructions = options.additionalInstructions;
	}
	if (typeof options?.moduleAccessCwd === "string") {
		out.moduleAccessCwd = options.moduleAccessCwd;
	} else {
		// Infer `moduleAccessCwd` from the first agent/tool descriptor so
		// `agent-os-client`'s `resolve_package_bin` can resolve the
		// adapter package's bin entry, AND the host's `node_modules` tree
		// can be projected via the module-access mount into the VM's
		// `/root/node_modules` without crossing symlinks that point
		// outside the mount root (the rivetkit pnpm layout symlinks
		// pi → /...../.pnpm/{key}/node_modules/@rivet-dev/agent-os-pi).
		// The packageDir already comes through as a realpath, so walking
		// up to the `node_modules` ancestor and taking its parent yields
		// a directory whose subtree is symlink-free under `node_modules`.
		const inferredCwd = inferModuleAccessCwd(rawSoftware);
		if (inferredCwd) out.moduleAccessCwd = inferredCwd;
	}
	if (Array.isArray(options?.loopbackExemptPorts)) {
		const ports = options.loopbackExemptPorts.filter(
			(p): p is number => typeof p === "number",
		);
		if (ports.length > 0) out.loopbackExemptPorts = ports;
	}
	if (Array.isArray(options?.allowedNodeBuiltins)) {
		const names = options.allowedNodeBuiltins.filter(
			(n): n is string => typeof n === "string",
		);
		if (names.length > 0) out.allowedNodeBuiltins = names;
	}

	return JSON.stringify(out);
}

interface AgentOsConfigJsonEnvelope {
	software?: AgentOsConfigJsonSoftwareEntry[];
	additionalInstructions?: string;
	moduleAccessCwd?: string;
	loopbackExemptPorts?: number[];
	allowedNodeBuiltins?: string[];
}

interface AgentOsConfigJsonSoftwareEntry {
	package: string;
	kind: "wasm-commands" | "agent" | "tool";
}

interface AgentOsOptionsLoose {
	software?: unknown[];
	additionalInstructions?: unknown;
	moduleAccessCwd?: unknown;
	loopbackExemptPorts?: unknown[];
	allowedNodeBuiltins?: unknown[];
}

/**
 * Map a single JS descriptor to the flat Rust shape, inferring `kind`
 * from the descriptor's structure. Returns `null` for descriptors that
 * carry no usable host path so the caller can drop them silently.
 */
/**
 * Walk up from a packageDir to the **outermost** `node_modules`
 * ancestor and return its parent. The outermost (not innermost) is
 * required because pnpm packages live at deep `.pnpm/{key}/node_modules/`
 * paths but their transitive deps live in sibling `.pnpm/{otherKey}/`
 * directories — a runtime `require()` from inside an agent package
 * needs the whole workspace-rooted `node_modules/.pnpm/` tree
 * projected, not just one keyed subdir.
 *
 * Returns `null` if no `node_modules` ancestor is found (defensive).
 */
function packageDirToModuleAccessCwd(packageDir: string): string | null {
	const segments = packageDir.split("/");
	for (let i = 0; i < segments.length; i++) {
		if (segments[i] === "node_modules") {
			return segments.slice(0, i).join("/") || "/";
		}
	}
	return null;
}

function inferModuleAccessCwd(
	rawSoftware: unknown[] | undefined,
): string | null {
	if (!Array.isArray(rawSoftware)) return null;
	for (const entry of rawSoftware) {
		const candidates = Array.isArray(entry) ? entry : [entry];
		for (const descriptor of candidates) {
			if (!descriptor || typeof descriptor !== "object") continue;
			const obj = descriptor as Record<string, unknown>;
			const type = obj.type;
			if (type !== "agent" && type !== "tool") continue;
			const packageDir = obj.packageDir;
			if (typeof packageDir !== "string" || packageDir.length === 0)
				continue;
			const cwd = packageDirToModuleAccessCwd(packageDir);
			if (cwd) return cwd;
		}
	}
	return null;
}

function mapSoftwareDescriptor(
	descriptor: unknown,
): AgentOsConfigJsonSoftwareEntry | null {
	if (!descriptor || typeof descriptor !== "object") return null;
	const obj = descriptor as Record<string, unknown>;

	// `commandDir` is the wasm-commands signal. Both
	// `WasmCommandSoftwareDescriptor` (typed) and `WasmCommandDirDescriptor`
	// (duck-typed registry packages) expose it, so we infer wasm-commands
	// from the field rather than the `type` discriminator.
	const commandDir = obj.commandDir;
	if (typeof commandDir === "string" && commandDir.length > 0) {
		return { package: commandDir, kind: "wasm-commands" };
	}

	// `packageDir` carries the host path for Agent/Tool descriptors.
	const packageDir = obj.packageDir;
	if (typeof packageDir === "string" && packageDir.length > 0) {
		const type = obj.type;
		if (type === "agent") {
			return { package: packageDir, kind: "agent" };
		}
		if (type === "tool") {
			return { package: packageDir, kind: "tool" };
		}
		// Has packageDir but unknown / missing type: not enough signal to
		// classify. Drop rather than guess.
	}

	return null;
}

function buildNativeFactoryBuilder<TConnParams>(
	parsed: AgentOsActorConfig<TConnParams>,
): (runtime: CoreRuntime) => ActorFactoryHandle {
	return (runtime) => {
		if (runtime.kind !== "napi") {
			throw new Error(
				`agentOs() is only supported on the native NAPI runtime (current runtime kind: ${runtime.kind})`,
			);
		}
		if (!runtime.createAgentOsFactory) {
			throw new Error(
				"runtime.createAgentOsFactory is not implemented on the active CoreRuntime",
			);
		}
		const options: NapiAgentOsOptions = {
			configJson: buildConfigJson(parsed),
		};
		return runtime.createAgentOsFactory(options, undefined);
	};
}

/**
 * Shorthand for the verbose `ActionContext` parametrization shared by
 * every typed action signature on the `agentOs(...)` actor. Each
 * action's first parameter is this context; the framework strips it
 * from the client-side surface so consumers call e.g.
 * `agent.readFile(path)`, not `agent.readFile(ctx, path)`.
 */
type AgentOsActionContext<TConnParams> = ActionContext<
	AgentOsActorState,
	TConnParams,
	undefined,
	AgentOsActorVars,
	undefined,
	DatabaseProvider<RawAccess>,
	AgentOsActorEvents,
	Record<never, never>
>;

/**
 * Result shape for `exec`. Matches the Rust `ExecResultDto` (camelCase
 * `exitCode`); not part of the upstream `agent-os-core` types because
 * the JS port exposed a richer record there.
 */
export interface AgentOsExecResult {
	exitCode: number;
	stdout: string;
	stderr: string;
}

/**
 * Result shape for `sendPrompt`. Matches the Rust `PromptReplyDto`.
 */
export interface AgentOsPromptReply {
	text: string;
	response: JsonRpcResponse;
}

/**
 * Result shape for `vmFetch`. The body comes through the wire as raw
 * bytes; the rivetkit client revives the `["$Uint8Array", base64]`
 * wrapper into a `Uint8Array` on the consumer side.
 */
export interface AgentOsVmFetchResult {
	status: number;
	body: Uint8Array;
}

/**
 * Result shape for `scheduleCron`.
 */
export interface AgentOsCronJobHandle {
	id: string;
}

/**
 * Action shape for `scheduleCron`. Tagged union mirroring the Rust
 * `CronActionArg` enum — only the wire-friendly variants are accepted.
 */
export type AgentOsCronActionInput =
	| { type: "exec"; command: string; args?: string[] }
	| { type: "session"; agentType: string; prompt: string };

/**
 * Option shape for `scheduleCron`.
 */
export interface AgentOsScheduleCronOptions {
	schedule: string;
	action: AgentOsCronActionInput;
	id?: string;
	overlap?: "allow" | "skip" | "queue";
}

/**
 * Typed action surface for the `agentOs(...)` actor. Each entry is the
 * server-side action signature with `c: ActionContext` as the first
 * parameter; the framework's `ActorActionMap` strips that first
 * parameter when constructing the client-facing surface so consumers
 * see `agent.readFile(path) => Promise<Uint8Array>`, etc.
 *
 * Notes on bytes vs strings on the consumer side:
 *  - `readFile` returns a `Uint8Array` (rivetkit's `JsonCompatAdapter`
 *    revives the `["$Uint8Array", base64]` wire wrapper into bytes).
 *  - `writeFile` / `writeFiles[].content` / `writeProcessStdin.data`
 *    accept either a `string` (UTF-8) or `Uint8Array`.
 *  - Process output broadcasts use a base64 string (`dataBase64`)
 *    because the broadcast pipe doesn't apply the byte-wrap uniformly
 *    across CBOR / JSON cells.
 */
type AgentOsActorActions<TConnParams> = {
	// Filesystem
	readFile: (
		c: AgentOsActionContext<TConnParams>,
		path: string,
	) => Promise<Uint8Array>;
	writeFile: (
		c: AgentOsActionContext<TConnParams>,
		path: string,
		content: string | Uint8Array,
	) => Promise<void>;
	stat: (
		c: AgentOsActionContext<TConnParams>,
		path: string,
	) => Promise<VirtualStat>;
	mkdir: (
		c: AgentOsActionContext<TConnParams>,
		path: string,
	) => Promise<void>;
	readdir: (
		c: AgentOsActionContext<TConnParams>,
		path: string,
	) => Promise<string[]>;
	exists: (
		c: AgentOsActionContext<TConnParams>,
		path: string,
	) => Promise<boolean>;
	move: (
		c: AgentOsActionContext<TConnParams>,
		from: string,
		to: string,
	) => Promise<void>;
	deleteFile: (
		c: AgentOsActionContext<TConnParams>,
		path: string,
	) => Promise<void>;
	readFiles: (
		c: AgentOsActionContext<TConnParams>,
		paths: string[],
	) => Promise<BatchReadResult[]>;
	writeFiles: (
		c: AgentOsActionContext<TConnParams>,
		entries: Array<{ path: string; content: string | Uint8Array }>,
	) => Promise<BatchWriteResult[]>;
	readdirRecursive: (
		c: AgentOsActionContext<TConnParams>,
		path: string,
	) => Promise<DirEntry[]>;

	// Process
	exec: (
		c: AgentOsActionContext<TConnParams>,
		command: string,
	) => Promise<AgentOsExecResult>;
	spawn: (
		c: AgentOsActionContext<TConnParams>,
		command: string,
		args?: string[],
	) => Promise<{ pid: number }>;
	waitProcess: (
		c: AgentOsActionContext<TConnParams>,
		pid: number,
	) => Promise<number>;
	killProcess: (
		c: AgentOsActionContext<TConnParams>,
		pid: number,
	) => Promise<void>;
	stopProcess: (
		c: AgentOsActionContext<TConnParams>,
		pid: number,
	) => Promise<void>;
	listProcesses: (
		c: AgentOsActionContext<TConnParams>,
	) => Promise<SpawnedProcessInfo[]>;
	allProcesses: (
		c: AgentOsActionContext<TConnParams>,
	) => Promise<ProcessInfo[]>;
	processTree: (
		c: AgentOsActionContext<TConnParams>,
	) => Promise<ProcessTreeNode[]>;
	getProcess: (
		c: AgentOsActionContext<TConnParams>,
		pid: number,
	) => Promise<SpawnedProcessInfo>;
	writeProcessStdin: (
		c: AgentOsActionContext<TConnParams>,
		pid: number,
		data: string | Uint8Array,
	) => Promise<void>;
	closeProcessStdin: (
		c: AgentOsActionContext<TConnParams>,
		pid: number,
	) => Promise<void>;

	// Session
	createSession: (
		c: AgentOsActionContext<TConnParams>,
		agentType: string,
		options?: Partial<CreateSessionOptions> & {
			skipOsInstructions?: boolean;
		},
	) => Promise<{ sessionId: string }>;
	sendPrompt: (
		c: AgentOsActionContext<TConnParams>,
		sessionId: string,
		text: string,
	) => Promise<AgentOsPromptReply>;
	listSessions: (
		c: AgentOsActionContext<TConnParams>,
	) => Promise<SessionInfo[]>;
	destroySession: (
		c: AgentOsActionContext<TConnParams>,
		sessionId: string,
	) => Promise<void>;
	closeSession: (
		c: AgentOsActionContext<TConnParams>,
		sessionId: string,
	) => Promise<void>;

	// Network
	vmFetch: (
		c: AgentOsActionContext<TConnParams>,
		port: number,
		url: string,
	) => Promise<AgentOsVmFetchResult>;

	// Cron
	scheduleCron: (
		c: AgentOsActionContext<TConnParams>,
		options: AgentOsScheduleCronOptions,
	) => Promise<AgentOsCronJobHandle>;
	listCronJobs: (
		c: AgentOsActionContext<TConnParams>,
	) => Promise<CronJobInfo[]>;
	cancelCronJob: (
		c: AgentOsActionContext<TConnParams>,
		id: string,
	) => Promise<void>;
};

/**
 * Type alias for the `agentOs(...)` return type. Events AND actions
 * are both typed at the TS surface so `agent.on("sessionEvent", ...)`
 * and `agent.readFile(path)` get correct payload / argument /
 * return-type inference and autocomplete.
 */
export type AgentOsActorDefinition<TConnParams> = ActorDefinition<
	AgentOsActorState,
	TConnParams,
	undefined,
	AgentOsActorVars,
	undefined,
	DatabaseProvider<RawAccess>,
	AgentOsActorEvents,
	Record<never, never>,
	AgentOsActorActions<TConnParams>
>;

export function agentOs<TConnParams = undefined>(
	config: AgentOsActorConfigInput<TConnParams>,
): AgentOsActorDefinition<TConnParams> {
	const parsed = agentOsActorConfigSchema.parse(
		config,
	) as AgentOsActorConfig<TConnParams>;

	// Construct a minimal definition through the existing actor() helper.
	// Pass the event tokens so the schema is registered with the framework
	// (mirrors what a hand-written `actor({...})` definition would do).
	// Then attach the Rust factory builder marker; no JS-side action ever
	// runs because the engine driver branches on `nativeFactoryBuilder`
	// before reaching the JS dispatch path.
	const definition = actor({
		actions: {},
		events: {
			sessionEvent: sessionEventToken,
			permissionRequest: permissionRequestToken,
			vmBooted: vmBootedToken,
			vmShutdown: vmShutdownToken,
			processOutput: processOutputToken,
			processExit: processExitToken,
			shellData: shellDataToken,
			cronEvent: cronEventToken,
		},
	}) as unknown as AgentOsActorDefinition<TConnParams>;
	definition.nativeFactoryBuilder = buildNativeFactoryBuilder(parsed);
	return definition;
}
