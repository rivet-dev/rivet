import type {
	AgentOsOptions,
	JsonRpcNotification,
	PermissionRequest,
} from "@rivet-dev/agent-os-core";
import { z } from "zod/v4";
import type { ActorContext, BeforeConnectContext } from "@/actor/contexts";
import type { DatabaseProvider } from "@/actor/database";
import type { RawAccess } from "@/db/config";
import type { AgentOsActorState, AgentOsActorVars } from "./types";

const zFunction = <
	T extends (...args: never[]) => unknown = (...args: unknown[]) => unknown,
>() =>
	z.custom<T>((val) => typeof val === "function", {
		message: "Expected a function",
	});

const AgentOsOptionsSchema = z.custom<AgentOsOptions>(
	(val) => typeof val === "object" && val !== null,
);

export const agentOsActorConfigSchema = z
	.object({
		options: AgentOsOptionsSchema.optional(),
		createOptions: zFunction().optional(),
		preview: z
			.object({
				defaultExpiresInSeconds: z.number().positive().default(3600),
				maxExpiresInSeconds: z.number().positive().default(86400),
			})
			.strict()
			.prefault(() => ({})),
		onBeforeConnect: zFunction().optional(),
		onSessionEvent: zFunction().optional(),
		onPermissionRequest: zFunction().optional(),
	})
	.strict()
	.refine(
		(data) =>
			(data.options !== undefined) !== (data.createOptions !== undefined),
		{
			message:
				"agentOs config must define exactly one of 'options' or 'createOptions'",
		},
	);

// --- Typed config types (generic callbacks overlaid on the Zod schema) ---

export type AgentOsContext<TConnParams> = ActorContext<
	AgentOsActorState,
	TConnParams,
	undefined,
	AgentOsActorVars,
	undefined,
	DatabaseProvider<RawAccess>
>;

interface AgentOsActorConfigCallbacks<TConnParams> {
	onBeforeConnect?: (
		c: BeforeConnectContext<
			AgentOsActorState,
			AgentOsActorVars,
			undefined,
			DatabaseProvider<RawAccess>
		>,
		params: TConnParams,
	) => void | Promise<void>;
	onSessionEvent?: (
		c: AgentOsContext<TConnParams>,
		sessionId: string,
		event: JsonRpcNotification,
	) => void | Promise<void>;
	onPermissionRequest?: (
		c: AgentOsContext<TConnParams>,
		sessionId: string,
		request: PermissionRequest,
	) => void | Promise<void>;
}

// Exclusive union: exactly one of `options` (static) or `createOptions`
// (per-actor-instance factory). Mirrors the sandboxActor pattern of
// `provider` / `createProvider`.
type AgentOsActorOptionsConfig<TConnParams> =
	| {
			/** Static VM options shared by all instances of this actor. Use
			 * `createOptions` instead if each instance needs its own sandbox,
			 * filesystem mounts, or per-instance configuration. */
			options: AgentOsOptions;
			createOptions?: never;
	  }
	| {
			options?: never;
			/** Factory called lazily on first VM access. Receives the actor
			 * context so options can vary per instance (e.g., dedicated
			 * sandboxes). Mutually exclusive with `options`. May be async. */
			createOptions: (
				c: AgentOsContext<TConnParams>,
			) => AgentOsOptions | Promise<AgentOsOptions>;
	  };

// Parsed config (after Zod defaults/transforms applied).
export type AgentOsActorConfig<TConnParams = undefined> = Omit<
	z.infer<typeof agentOsActorConfigSchema>,
	| "options"
	| "createOptions"
	| "onBeforeConnect"
	| "onSessionEvent"
	| "onPermissionRequest"
> &
	AgentOsActorConfigCallbacks<TConnParams> &
	AgentOsActorOptionsConfig<TConnParams>;

// Input config (what users pass in before Zod transforms).
export type AgentOsActorConfigInput<TConnParams = undefined> = Omit<
	z.input<typeof agentOsActorConfigSchema>,
	| "options"
	| "createOptions"
	| "onBeforeConnect"
	| "onSessionEvent"
	| "onPermissionRequest"
> &
	AgentOsActorConfigCallbacks<TConnParams> &
	AgentOsActorOptionsConfig<TConnParams>;
