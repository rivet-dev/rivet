// Core API
export {
  createRivetKit,
  createRivetKitWithClient,
  createClient,
  type RivetKit,
  type ActorState,
  type ReactiveActorHandle,
  type ActorConnStatus,
  type ActorOptions,
  type AnyActorRegistry,
  type WarmUpActorOptions,
  type PreConnectHandle,
  // Deprecated: renamed to WarmUpActorOptions.
  type PreloadActorOptions,
  type ActionDefaults,
  type SvelteRivetKitOptions,
  type SvelteActorOptions,
} from "./rivetkit.svelte.js";

// Context helpers
export { createRivetContext, type RivetContext } from "./context.js";

// Shared client / mixed-mode helpers
export {
  createSharedRivetKit,
  withActorParams,
  createReactiveConnection,
  type ReactiveConnection,
  type ReactiveConnectionSource,
} from "./shared.svelte.js";

// Connection health
export {
  createConnectionHealth,
  type ConnectionSource,
  type ConnectionHealth,
  type ActorHealth,
  type HealthStatus,
} from "./connection-health.svelte.js";

// Opt-in live connection registry (local-dev overlays)
export {
  createConnectionInspector,
  normalizeActorKey,
  fallbackInspectorHash,
  CONNECTION_INSPECTOR_SAMPLE_KEYS,
  type ConnectionInspector,
  type ConnectionInspectorSample,
  type ConnectionInspectorReport,
} from "./connection-inspector.svelte.js";

// Error utilities
export {
  isActorError,
  actorErrorCode,
  actorErrorMessage,
  getActionError,
  type ActorErrorLike,
  type ActionErrorInfo,
} from "./errors.js";

// Ecosystem-standard types (runed / melt-ui / bits-ui convention)
export type { Getter, MaybeGetter } from "./internal/types.js";
export { extract } from "./internal/extract.js";
