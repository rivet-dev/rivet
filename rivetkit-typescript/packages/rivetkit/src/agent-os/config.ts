import type {
	AgentOsOptions,
	JsonRpcNotification,
	PermissionRequest,
} from "@rivet-dev/agent-os-core";
import { z } from "zod/v4";
import type { ActorContext, BeforeConnectContext } from "@/actor/config";
import type { AgentOsActorState, AgentOsActorVars } from "./types";

const zFunction = <
	T extends (...args: any[]) => any = (...args: unknown[]) => unknown,
>() => z.custom<T>((val) => typeof val === "function");

const AgentOsOptionsSchema = z.custom<AgentOsOptions>(
	(val) => typeof val === "object" && val !== null,
);
// Layerr: accept either a static options object OR a `(c) => AgentOsOptions`
// factory so the caller can compute per-actor configuration (e.g. a mount
// path bound to `c.key`). The factory runs once per actor instance inside
// `ensureVm`, with the actor's live context.
const AgentOsOptionsFactorySchema = z.custom<
	(c: any) => AgentOsOptions | Promise<AgentOsOptions>
>((val) => typeof val === "function");
const AgentOsOptionsOrFactorySchema = z.union([
	AgentOsOptionsSchema,
	AgentOsOptionsFactorySchema,
]);

export const agentOsActorConfigSchema = z
	.object({
		options: AgentOsOptionsOrFactorySchema.optional(),
		preview: z
			.object({
				defaultExpiresInSeconds: z.number().positive().default(3600),
				maxExpiresInSeconds: z.number().positive().default(86400),
			})
			.strict()
			.prefault(() => ({})),
		// Layerr: let callers override agent-os's hardcoded graceful-shutdown
		// window + per-action timeout (both default to 900_000 in the actor
		// `options` block in ./actor/index.ts). v2.3.0's `sleepGracePeriod`
		// now bounds the entire graceful shutdown window for sleep AND destroy,
		// subsuming the old layerr `runStopTimeout` drain-budget patch (dropped).
		sleepGracePeriod: z.number().nonnegative().optional(),
		actionTimeout: z.number().nonnegative().optional(),
		// Layerr: user-supplied actions merged on top of the built-in agent-os
		// action set (e.g. gitLog/gitDiff in workspace-git.ts). strict() rejects
		// unknown keys, so this slot is required to expose actor RPCs without
		// forking agentOs().
		actions: z.record(z.string(), zFunction()).optional(),
		onBeforeConnect: zFunction().optional(),
		onSessionEvent: zFunction().optional(),
		onPermissionRequest: zFunction().optional(),
	})
	.strict();

// --- Typed config types (generic callbacks overlaid on the Zod schema) ---

type AgentOsActorContext<TConnParams> = ActorContext<
	AgentOsActorState,
	TConnParams,
	undefined,
	AgentOsActorVars,
	undefined,
	any
>;

interface AgentOsActorConfigCallbacks<TConnParams> {
	onBeforeConnect?: (
		c: BeforeConnectContext<
			AgentOsActorState,
			AgentOsActorVars,
			undefined,
			any
		>,
		params: TConnParams,
	) => void | Promise<void>;
	onSessionEvent?: (
		c: AgentOsActorContext<TConnParams>,
		sessionId: string,
		event: JsonRpcNotification,
	) => void | Promise<void>;
	onPermissionRequest?: (
		c: AgentOsActorContext<TConnParams>,
		sessionId: string,
		request: PermissionRequest,
	) => void | Promise<void>;
}

// `options` either as a static `AgentOsOptions` object OR as a factory
// `(c) => AgentOsOptions` resolved per actor instance inside `ensureVm`.
export type AgentOsOptionsOrFactory<TConnParams> =
	| AgentOsOptions
	| ((
			c: AgentOsActorContext<TConnParams>,
	  ) => AgentOsOptions | Promise<AgentOsOptions>);

interface AgentOsActorConfigOptions<TConnParams> {
	options?: AgentOsOptionsOrFactory<TConnParams>;
}

// Parsed config (after Zod defaults/transforms applied).
export type AgentOsActorConfig<TConnParams = undefined> = Omit<
	z.infer<typeof agentOsActorConfigSchema>,
	"onBeforeConnect" | "onSessionEvent" | "onPermissionRequest" | "options"
> &
	AgentOsActorConfigOptions<TConnParams> &
	AgentOsActorConfigCallbacks<TConnParams>;

// Input config (what users pass in before Zod transforms).
export type AgentOsActorConfigInput<TConnParams = undefined> = Omit<
	z.input<typeof agentOsActorConfigSchema>,
	"onBeforeConnect" | "onSessionEvent" | "onPermissionRequest" | "options"
> &
	AgentOsActorConfigOptions<TConnParams> &
	AgentOsActorConfigCallbacks<TConnParams>;
