/**
 * Rust-backed `agentOs(...)` definition (Phase 1c+).
 *
 * Produces an `ActorDefinition` whose `nativeFactoryBuilder` constructs a
 * `CoreActorFactory` through `runtime.createAgentOsFactory(...)` (NAPI →
 * `rivetkit_agent_os::build_core_factory`). All lifecycle, state, and
 * action dispatch live in the Rust crate. The JS shim only validates
 * configuration and hands it across the bridge.
 */

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
	const definition = actor({
		actions: {},
	}) as unknown as AgentOsActorDefinition<TConnParams>;
	definition.nativeFactoryBuilder = buildNativeFactoryBuilder(parsed);
	return definition;
}
