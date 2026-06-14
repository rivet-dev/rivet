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
	flattenSoftware((parsed.options as { software?: unknown })?.software, descriptors);

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

	// `moduleAccessCwd` backs `/root/node_modules` in the VM (agent SDK + transitive
	// dep resolution). It defaults to the server's cwd, but can be pointed at a
	// pre-generated flat node_modules via `AGENT_OS_MODULE_ACCESS_CWD` so pnpm-isolated
	// workspaces can mount a hoisted tree without restructuring the whole workspace.
	const moduleAccessCwd =
		process.env.AGENT_OS_MODULE_ACCESS_CWD ?? process.cwd();
	return JSON.stringify({ software, moduleAccessCwd });
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
	const definition = actor({
		actions: {},
	}) as unknown as AgentOsActorDefinition<TConnParams>;
	definition.nativeFactoryBuilder = buildNativeFactoryBuilder(parsed);
	return definition;
}
