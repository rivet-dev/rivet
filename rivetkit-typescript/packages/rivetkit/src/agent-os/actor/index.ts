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
 * Build the JSON envelope the Rust crate consumes. Only the subset that
 * is currently serializable across the bridge is included; the rest of
 * `AgentOsActorConfig` (callbacks, preview window, tool kits) lands in
 * later phases. The Rust deserializer uses `deny_unknown_fields`, so the
 * envelope must stay in lock-step with `agent_os.rs::AgentOsConfigJson`.
 *
 * `software` is intentionally NOT threaded yet. The JS `SoftwareInput`
 * union (`name`, `type`, `commandDir`, agent configs) and the Rust
 * client's `{package, version}` shape mean a naive `name` extraction
 * makes the Rust sidecar look for `./node_modules/<name>` — but the real
 * npm package is `@rivet-dev/agent-os-<name>`. A correct mapping needs
 * the host-side `commandDir` or the full npm package name. Phase 3 will
 * resolve this when `exec`/`spawn` actually need installed software.
 *
 * The Rust-side parsing path is verified by
 * `rivetkit-napi/tests/agent_os_factory.rs::parsing` for non-empty
 * configs, so this function can be expanded without rewiring the bridge.
 */
export function buildConfigJson<TConnParams>(
	_parsed: AgentOsActorConfig<TConnParams>,
): string {
	return "{}";
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
