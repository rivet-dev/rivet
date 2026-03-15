import { z } from "zod/v4";
import type {
	ActorContext,
	BeforeConnectContext,
} from "@/actor/contexts";
import type { AnyDatabaseProvider } from "@/actor/database";
import type {
	PermissionRequestListener,
	SessionEventListener,
} from "sandbox-agent";
import type {
	SandboxActorProvider,
	SandboxActorVars,
	SandboxActorState,
} from "./types";

const zFunction = <
	T extends (...args: any[]) => any = (...args: unknown[]) => unknown,
>() => z.custom<T>((val) => typeof val === "function");

const SandboxActorProviderSchema = z.object({
	name: z.string(),
	create: zFunction<SandboxActorProvider["create"]>(),
	destroy: zFunction<SandboxActorProvider["destroy"]>(),
	connectAgent: zFunction<SandboxActorProvider["connectAgent"]>(),
	wake: zFunction<NonNullable<SandboxActorProvider["wake"]>>().optional(),
});

export const SandboxActorOptionsSchema = z
	.object({
		// Log if the actor still thinks a turn is active but no new session event
		// has arrived for this long.
		warningAfterMs: z.number().nonnegative().default(30_000),
		// Clear active-turn state after this timeout so a missing terminal event
		// cannot keep the actor awake forever.
		staleAfterMs: z.number().positive().default(5 * 60_000),
	})
	.strict()
	.prefault(() => ({}))
	.transform((value) => ({
		...value,
		warningAfterMs: Math.min(value.warningAfterMs, value.staleAfterMs),
	}));

export type SandboxActorOptions = z.input<typeof SandboxActorOptionsSchema>;
export type SandboxActorOptionsRuntime = z.infer<
	typeof SandboxActorOptionsSchema
>;

// This schema validates the config at runtime. Generic callback types are
// defined separately below following the same pattern as ActorConfigSchema:
// infer from the schema, omit function keys, then intersect typed callbacks.
export const SandboxActorConfigSchema = z
	.object({
		provider: SandboxActorProviderSchema.optional(),
		createProvider: zFunction().optional(),
		persistRawEvents: z.boolean().optional(),
		destroyActor: z.boolean().default(false),
		options: SandboxActorOptionsSchema,
		onBeforeConnect: zFunction().optional(),
		onSessionEvent: zFunction().optional(),
		onPermissionRequest: zFunction().optional(),
	})
	.strict()
	.refine(
		(data) =>
			(data.provider !== undefined) !==
			(data.createProvider !== undefined),
		{
			message:
				"Sandbox actor config must define exactly one of 'provider' or 'createProvider'",
		},
	);

// --- Typed config types (generic callbacks overlaid on the Zod schema) ---

type SandboxActorContext<TConnParams> = ActorContext<
	SandboxActorState,
	TConnParams,
	undefined,
	SandboxActorVars,
	undefined,
	AnyDatabaseProvider
>;

interface SandboxActorConfigCallbacks<TConnParams> {
	onBeforeConnect?: (
		c: BeforeConnectContext<
			SandboxActorState,
			SandboxActorVars,
			undefined,
			AnyDatabaseProvider
		>,
		params: TConnParams,
	) => void | Promise<void>;
	onSessionEvent?: (
		c: SandboxActorContext<TConnParams>,
		sessionId: string,
		event: Parameters<SessionEventListener>[0],
	) => void | Promise<void>;
	onPermissionRequest?: (
		c: SandboxActorContext<TConnParams>,
		sessionId: string,
		request: Parameters<PermissionRequestListener>[0],
	) => void | Promise<void>;
}

type SandboxActorProviderConfig<TConnParams> =
	| {
			provider: SandboxActorProvider;
			createProvider?: never;
	  }
	| {
			provider?: never;
			createProvider: (
				c: SandboxActorContext<TConnParams>,
			) => SandboxActorProvider | Promise<SandboxActorProvider>;
	  };

// Parsed config (after Zod defaults/transforms applied).
export type SandboxActorConfig<TConnParams = undefined> = Omit<
	z.infer<typeof SandboxActorConfigSchema>,
	| "provider"
	| "createProvider"
	| "onBeforeConnect"
	| "onSessionEvent"
	| "onPermissionRequest"
> &
	SandboxActorConfigCallbacks<TConnParams> &
	SandboxActorProviderConfig<TConnParams>;

// Input config (what users pass in before Zod transforms).
export type SandboxActorConfigInput<TConnParams = undefined> = Omit<
	z.input<typeof SandboxActorConfigSchema>,
	| "provider"
	| "createProvider"
	| "onBeforeConnect"
	| "onSessionEvent"
	| "onPermissionRequest"
> &
	SandboxActorConfigCallbacks<TConnParams> &
	SandboxActorProviderConfig<TConnParams>;

export type SandboxActorBeforeConnectContext<TConnParams = undefined> =
	BeforeConnectContext<
		SandboxActorState,
		SandboxActorVars,
		undefined,
		AnyDatabaseProvider
	>;
