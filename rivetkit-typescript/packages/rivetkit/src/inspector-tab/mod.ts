/**
 * Type and schema declarations for custom Rivet inspector tabs.
 *
 * Everything here is **derived from runtime Zod schemas** that the
 * dashboard and rivetkit itself use, so the published types cannot
 * drift from the wire format. There are three sources of truth:
 *
 *   1. The `inspector.tabs[]` actor-config Zod schemas in
 *      `../actor/config.ts` — re-exported below. These validate the
 *      authoring surface (`defineActor({ inspector: { tabs: [...] } })`).
 *   2. The dashboard ↔ tab postMessage envelopes defined in this file.
 *      The inspector-ui SPA (`frontend/apps/inspector-ui/src/bridge.ts`)
 *      imports the same schemas to validate messages at runtime, so any
 *      change here lands on both ends in lockstep.
 *   3. The inspector HTTP endpoint response schemas defined in this
 *      file. The dashboard parses every response through them; if the
 *      Rust handler ever emits a different shape, the dashboard fails
 *      loudly at the parse step instead of silently casting `unknown`.
 *
 * Tab bundles import these types for autocomplete and compile-time
 * safety:
 *
 * ```ts
 * import type {
 *   ShellToTabMessage,
 *   TabToShellMessage,
 *   V1Init,
 *   InspectorStateResponse,
 * } from "rivetkit/inspector-tab";
 * ```
 */

import { z } from "zod";

// ============================================================================
// Re-exported from `../actor/config.ts` — the authoring schema for
// `defineActor({ inspector: { tabs: [...] } })`.
// ============================================================================

export type { ActorInspectorConfig } from "../actor/config";
export {
	ActorInspectorConfigSchema,
	BUILTIN_INSPECTOR_TAB_IDS,
	BuiltinInspectorTabIdSchema,
	CustomInspectorTabEntrySchema,
	HideInspectorTabEntrySchema,
	InspectorTabEntrySchema,
} from "../actor/config";

import type {
	CustomInspectorTabEntrySchema,
	HideInspectorTabEntrySchema,
	InspectorTabEntrySchema,
} from "../actor/config";

/** One entry in the actor's `inspector.tabs[]` declaration. */
export type ActorInspectorTabEntry = z.input<typeof InspectorTabEntrySchema>;
/** A custom-tab entry — adds a new tab to the dashboard strip. */
export type ActorCustomInspectorTabEntry = z.input<
	typeof CustomInspectorTabEntrySchema
>;
/** A hide modifier — removes a built-in tab from the strip. */
export type ActorHideInspectorTabEntry = z.input<
	typeof HideInspectorTabEntrySchema
>;
/** Union of the six built-in inspector tab ids. */
export type BuiltinInspectorTabId =
	| "workflow"
	| "database"
	| "state"
	| "queue"
	| "schedules"
	| "connections"
	| "console";

// ============================================================================
// Dashboard ↔ tab postMessage envelope (source of truth).
//
// These Zod schemas are imported by `frontend/apps/inspector-ui/src/bridge.ts`
// and used to validate inbound messages at runtime. Changes here flow to
// both sides in a single commit.
// ============================================================================

/** Public tab descriptor that crosses the dashboard ↔ inspector-ui bridge. */
export const InspectorTabDescriptorSchema = z.object({
	id: z.string(),
	label: z.string(),
	icon: z.string(),
	/**
	 * `true` for author-shipped custom tabs; absent or `false` for
	 * built-in tabs the SPA renders. Lets the dashboard route the
	 * iframe `src` to `/inspector/custom-tabs/<id>/` for custom tabs and
	 * `/inspector/ui/` for built-ins without the dashboard needing to
	 * know which ids are built-in.
	 */
	isCustom: z.boolean().optional(),
});

/**
 * Initial handshake from the dashboard. Sent on first mount and again on
 * every token refresh. Tabs MUST accept late `init` messages and replace
 * the cached token.
 */
export const V1InitSchema = z.object({
	type: z.literal("init"),
	v: z.literal(1),
	/** The actor this tab is mounted for. */
	actorId: z.string(),
	/**
	 * Per-actor inspector bearer token. Tabs include it as
	 * `Authorization: Bearer ${authToken}` on every authenticated fetch.
	 */
	authToken: z.string(),
	/**
	 * Outer Rivet API token. Optional; not required for inspector HTTP
	 * routes but available to tabs that want to call the engine REST API.
	 */
	rivetToken: z.string().optional(),
	/**
	 * The tab id the dashboard wants active at mount time. Multi-view
	 * tabs may read this to seed their initial route; most tabs ignore it.
	 */
	activeTab: z.string().optional(),
	/**
	 * Dashboard's currently active theme. Tabs that use the shared
	 * stylesheet (`/inspector/tab.css`) mirror it by toggling the `dark`
	 * class on `<html>`. Optional for backwards compatibility — tabs
	 * should default to `"dark"` if absent (the dashboard pinned dark
	 * mode before this field was added).
	 */
	theme: z.enum(["light", "dark"]).optional(),
});

/**
 * Dashboard tells the inspector-ui SPA which built-in tab to render. Not
 * sent to custom tabs — when the user activates a custom tab the
 * dashboard navigates the outer iframe to a different `src`.
 */
export const V1SetActiveTabSchema = z.object({
	type: z.literal("set-active-tab"),
	v: z.literal(1),
	tab: z.string(),
});

/**
 * Tab → dashboard. Sent once after the message listener is registered.
 * The dashboard hides the "Connecting to inspector…" overlay on receipt;
 * if it never arrives the overlay times out after 8 s.
 */
export const V1ReadySchema = z.object({
	type: z.literal("ready"),
	v: z.literal(1),
});

/**
 * Tab → dashboard. Sent when the tab gets a 401 on an inspector data
 * call. The dashboard refreshes the token and re-issues `v1Init`.
 */
export const V1TokenRefreshNeededSchema = z.object({
	type: z.literal("token-refresh-needed"),
	v: z.literal(1),
});

/**
 * Emitted by the inspector-ui SPA to tell the dashboard which tabs to
 * render in the strip. Custom tab bundles do NOT emit this — only the
 * SPA does.
 */
export const V1TabsAvailableSchema = z.object({
	type: z.literal("tabs-available"),
	v: z.literal(1),
	tabs: z.array(InspectorTabDescriptorSchema),
});

/** Discriminated union of messages a tab can RECEIVE from the dashboard. */
export const ShellToTabMessageSchema = z.discriminatedUnion("type", [
	V1InitSchema,
	V1SetActiveTabSchema,
]);

/** Discriminated union of messages a tab can SEND to the dashboard. */
export const TabToShellMessageSchema = z.discriminatedUnion("type", [
	V1ReadySchema,
	V1TabsAvailableSchema,
	V1TokenRefreshNeededSchema,
]);

export type InspectorTabDescriptor = z.infer<
	typeof InspectorTabDescriptorSchema
>;
export type V1Init = z.infer<typeof V1InitSchema>;
export type V1SetActiveTab = z.infer<typeof V1SetActiveTabSchema>;
export type V1Ready = z.infer<typeof V1ReadySchema>;
export type V1TokenRefreshNeeded = z.infer<typeof V1TokenRefreshNeededSchema>;
export type V1TabsAvailable = z.infer<typeof V1TabsAvailableSchema>;
export type ShellToTabMessage = z.infer<typeof ShellToTabMessageSchema>;
export type TabToShellMessage = z.infer<typeof TabToShellMessageSchema>;

/** Stable envelope protocol version. Bump only when introducing a v2 shape. */
export const POSTMESSAGE_PROTOCOL_VERSION = 1;

/** URL query parameters the dashboard sets on the tab iframe `src`. */
export const SHELL_ORIGIN_PARAM = "shellOrigin";
export const ACTOR_ID_PARAM = "actorId";

// ============================================================================
// Inspector HTTP endpoint response schemas (source of truth).
//
// The dashboard parses every authenticated inspector response through
// these schemas (`frontend/src/components/actors/actor-inspector-context.tsx`).
// If the Rust handler emits a different shape the parse fails and the
// dashboard surfaces an error — drift is loud.
// ============================================================================

/** `GET /inspector/state` response shape. */
export const InspectorStateResponseSchema = z.object({
	/** Current actor state (whatever shape the actor declared). */
	state: z.unknown(),
	/**
	 * `false` when the actor did not declare any state — `state` is then
	 * `null` and `PATCH /inspector/state` returns an error.
	 */
	isStateEnabled: z.boolean(),
});

/** `POST /inspector/action/<name>` request body. */
export const InspectorActionRequestSchema = z
	.object({
		/** Positional arguments. Mutually exclusive with `properties`. */
		args: z.array(z.unknown()).optional(),
		/** Keyed arguments. Mutually exclusive with `args`. */
		properties: z.record(z.string(), z.unknown()).optional(),
	})
	.refine(
		(body) => !(body.args !== undefined && body.properties !== undefined),
		"Use either `args` or `properties`, not both",
	);

/** `POST /inspector/action/<name>` response shape. */
export const InspectorActionResponseSchema = z.object({
	output: z.unknown(),
});

/** `GET /inspector/rpcs` response shape — the list of action names. */
export const InspectorRpcsResponseSchema = z.object({
	rpcs: z.array(z.string()),
});

/** One connection record in `GET /inspector/connections`. */
export const InspectorConnectionSchema = z.object({
	connectionType: z.string().nullable(),
	id: z.string(),
	details: z.object({
		connectionType: z.string().nullable(),
		params: z.unknown(),
		stateEnabled: z.boolean(),
		state: z.unknown(),
		subscriptions: z.number(),
		isHibernatable: z.boolean(),
	}),
});

/** `GET /inspector/connections` response shape. */
export const InspectorConnectionsResponseSchema = z.object({
	connections: z.array(InspectorConnectionSchema),
});

/** One queued message in `GET /inspector/queue`. */
export const InspectorQueueMessageSchema = z.object({
	id: z.string(),
	name: z.string(),
	createdAtMs: z.number(),
});

/** `GET /inspector/queue` response shape. */
export const InspectorQueueResponseSchema = z.object({
	size: z.number(),
	maxSize: z.number(),
	/** `true` if `?limit=N` truncated the message list below `size`. */
	truncated: z.boolean(),
	messages: z.array(InspectorQueueMessageSchema),
});

export type InspectorStateResponse = z.infer<
	typeof InspectorStateResponseSchema
>;
export type InspectorActionRequest = z.infer<
	typeof InspectorActionRequestSchema
>;
export type InspectorActionResponse = z.infer<
	typeof InspectorActionResponseSchema
>;
export type InspectorRpcsResponse = z.infer<typeof InspectorRpcsResponseSchema>;
export type InspectorConnection = z.infer<typeof InspectorConnectionSchema>;
export type InspectorConnectionsResponse = z.infer<
	typeof InspectorConnectionsResponseSchema
>;
export type InspectorQueueMessage = z.infer<typeof InspectorQueueMessageSchema>;
export type InspectorQueueResponse = z.infer<
	typeof InspectorQueueResponseSchema
>;
