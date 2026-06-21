/**
 * Rust-backed `agentOs(...)` definition (Phase 1c+).
 *
 * Produces an `ActorDefinition` whose `nativeFactoryBuilder` constructs a
 * `CoreActorFactory` through `runtime.createAgentOsFactory(...)` (NAPI →
 * `rivetkit_agent_os::build_core_factory`). All lifecycle, state, and
 * action dispatch live in the Rust crate. The JS shim only validates
 * configuration and hands it across the bridge.
 */

import { getSidecarPath } from "@rivet-dev/agent-os-sidecar";
import { actor, type ActorDefinition } from "@/actor/mod";
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
import type { AgentOsActorState, AgentOsActorVars } from "../types";

/**
 * Build the JSON envelope the Rust crate consumes. The Rust deserializer
 * uses `deny_unknown_fields`, so the envelope must stay in lock-step
 * with `agent_os.rs::AgentOsConfigJson`.
 *
 * Software threading: each software descriptor is flattened (meta packages
 * such as `common` are arrays of descriptors) and mapped to the Rust
 * `SoftwareInput { package, kind }`. The agent-os-client resolves an
 * ABSOLUTE `package` directly (its `resolve_software` lets an absolute path
 * bypass the `node_modules` prefix), so the descriptor's already-resolved
 * `commandDir` (wasm commands) / `packageDir` (agents/tools) is forwarded as
 * `package`. `build_command_mounts` then mounts each wasm dir at
 * `/__agentos/commands/{N}/`, which is what makes `exec`/shell work.
 */
interface SoftwareDescriptorLike {
	commandDir?: string;
	packageDir?: string;
	agent?: unknown;
	hostTool?: unknown;
	toolkit?: unknown;
}

interface NativeMountLike {
	path: string;
	plugin: {
		id: string;
		config?: unknown;
	};
	readOnly?: boolean;
}

/**
 * A native `host_dir` mount of a host `node_modules` directory at
 * `/root/node_modules`, the serializable form `agentOs({ options: { mounts } })`
 * accepts across the NAPI boundary.
 */
export interface NodeModulesMountConfig {
	path: "/root/node_modules";
	plugin: { id: "host_dir"; config: { hostPath: string; readOnly: boolean } };
	readOnly: boolean;
}

/**
 * Mount a host `node_modules` directory into the VM at `/root/node_modules`.
 *
 * This is the explicit, mount-based replacement for the removed `moduleAccessCwd`
 * / `AGENT_OS_MODULE_ACCESS_CWD` mechanism: the VM module resolver reads the
 * mounted tree through the kernel VFS, so the caller supplies exactly the
 * `node_modules` directory whose packages should resolve in the guest.
 *
 * @param hostNodeModulesDir Absolute host path to a `node_modules` directory.
 * @param opts.readOnly Defaults to `true`; the mount is read-only.
 */
export function nodeModulesMount(
	hostNodeModulesDir: string,
	opts?: { readOnly?: boolean },
): NodeModulesMountConfig {
	const readOnly = opts?.readOnly ?? true;
	return {
		path: "/root/node_modules",
		plugin: {
			id: "host_dir",
			config: { hostPath: hostNodeModulesDir, readOnly },
		},
		readOnly,
	};
}

function flattenSoftware(input: unknown, out: SoftwareDescriptorLike[]): void {
	if (input == null) return;
	if (Array.isArray(input)) {
		for (const item of input) flattenSoftware(item, out);
		return;
	}
	if (typeof input === "object") out.push(input as SoftwareDescriptorLike);
}

export function buildConfigJson<TConnParams>(
	parsed: AgentOsActorConfig<TConnParams>,
): string {
	const descriptors: SoftwareDescriptorLike[] = [];
	flattenSoftware(
		(parsed.options as { software?: unknown })?.software,
		descriptors,
	);

	const software: Array<{ package: string; kind?: string }> = [];
	for (const d of descriptors) {
		if (typeof d.commandDir === "string") {
			// Wasm command directory (kind defaults to WasmCommands on the Rust side).
			software.push({ package: d.commandDir });
		} else if (typeof d.packageDir === "string") {
			// Agent SDK / host-tool package: forwarded but not mounted as commands.
			// `kind` matches the kebab-case serde tags of the Rust `SoftwareKind`
			// enum (`wasm-commands` / `agent` / `tool`).
			software.push({
				package: d.packageDir,
				kind: d.hostTool || d.toolkit ? "tool" : "agent",
			});
		}
	}

	// `/root/node_modules` (agent SDK + transitive dep resolution) is now supplied
	// explicitly by the client via `options.mounts` (see `nodeModulesMount(...)`),
	// not derived from a host cwd. The VM module resolver reads the mounted tree
	// through the kernel VFS. There is no `moduleAccessCwd` / `AGENT_OS_MODULE_ACCESS_CWD`.
	const options = (parsed.options ?? {}) as Record<string, unknown>;
	const mounts = serializeNativeMounts(options.mounts);
	const sidecar = serializeSidecar(options.sidecar);
	return JSON.stringify({
		software,
		additionalInstructions: options.additionalInstructions,
		loopbackExemptPorts: options.loopbackExemptPorts,
		allowedNodeBuiltins: options.allowedNodeBuiltins,
		permissions: options.permissions,
		rootFilesystem: options.rootFilesystem,
		mounts,
		limits: options.limits,
		sidecar,
	});
}

function serializeNativeMounts(input: unknown): NativeMountLike[] | undefined {
	if (input == null) return undefined;
	if (!Array.isArray(input)) {
		throw new Error("agentOs() options.mounts must be an array");
	}
	return input.map((mount, index) => {
		if (!mount || typeof mount !== "object") {
			throw new Error(
				`agentOs() options.mounts[${index}] must be an object`,
			);
		}
		const record = mount as Record<string, unknown>;
		if (record.driver !== undefined) {
			throw new Error(
				"agentOs() only supports Native mounts across the NAPI boundary; Plain mounts with driver callbacks are not serializable",
			);
		}
		if (record.filesystem !== undefined) {
			throw new Error(
				"agentOs() only supports Native mounts across the NAPI boundary; Overlay mounts are not serializable",
			);
		}
		const plugin = record.plugin;
		if (
			typeof record.path !== "string" ||
			!plugin ||
			typeof plugin !== "object" ||
			typeof (plugin as Record<string, unknown>).id !== "string"
		) {
			throw new Error(
				`agentOs() options.mounts[${index}] must be a Native mount with { path, plugin: { id, config? } }`,
			);
		}
		return {
			path: record.path,
			plugin: {
				id: (plugin as Record<string, unknown>).id as string,
				config: (plugin as Record<string, unknown>).config,
			},
			readOnly:
				typeof record.readOnly === "boolean"
					? record.readOnly
					: undefined,
		};
	});
}

function serializeSidecar(input: unknown): { pool?: string } | undefined {
	if (input == null) return undefined;
	if (!input || typeof input !== "object") {
		throw new Error("agentOs() options.sidecar must be an object");
	}
	const record = input as Record<string, unknown>;
	if (record.kind === "explicit" || record.handle !== undefined) {
		throw new Error(
			"agentOs() only supports sidecar shared pool configuration across the NAPI boundary; explicit sidecar handles are not serializable",
		);
	}
	if (record.kind !== undefined && record.kind !== "shared") {
		throw new Error('agentOs() options.sidecar.kind must be "shared"');
	}
	return typeof record.pool === "string" ? { pool: record.pool } : {};
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
			// Resolve the prebuilt sidecar binary from the npm package and pass
			// it through to the agent-os client so it spawns the bundled binary
			// rather than relying on `agent-os-sidecar` being on PATH.
			sidecarBinaryPath: getSidecarPath(),
		};
		return runtime.createAgentOsFactory(options, undefined);
	};
}

/**
 * Type alias for the `agentOs(...)` return type. Events are not typed at
 * the TS surface because the Rust factory owns the broadcast set and the
 * test/client surface uses `any` for actions.
 */
export type AgentOsActorDefinition<TConnParams> = ActorDefinition<
	AgentOsActorState,
	TConnParams,
	undefined,
	AgentOsActorVars,
	undefined,
	DatabaseProvider<RawAccess>,
	Record<never, never>,
	Record<never, never>,
	any
>;

export function agentOs<TConnParams = undefined>(
	config: AgentOsActorConfigInput<TConnParams>,
): AgentOsActorDefinition<TConnParams> {
	const parsed = agentOsActorConfigSchema.parse(
		config,
	) as AgentOsActorConfig<TConnParams>;

	// Construct a minimal definition through the existing actor() helper,
	// then attach the Rust factory builder marker. The actions block stays
	// empty because no JS-side action ever runs: the engine driver branches
	// on `nativeFactoryBuilder` before reaching the JS dispatch path.
	//
	// `actorOptions` (e.g. `sleepTimeout`, `noSleep`) is forwarded as the
	// actor `options` block so `buildActorConfig` threads it to the engine
	// sleep timer; this is what lets a caller make the actor sleep quickly so
	// the VM is torn down and sessions resume lazily on the next prompt.
	const actorOptions = (parsed as { actorOptions?: Record<string, unknown> })
		.actorOptions;
	const definition = actor({
		actions: {},
		...(actorOptions ? { options: actorOptions } : {}),
	} as Parameters<
		typeof actor
	>[0]) as unknown as AgentOsActorDefinition<TConnParams>;
	definition.nativeFactoryBuilder = buildNativeFactoryBuilder(parsed);
	return definition;
}
