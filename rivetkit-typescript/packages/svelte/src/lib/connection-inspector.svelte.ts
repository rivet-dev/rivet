/**
 * Opt-in registry of package-managed Rivet actor connections.
 *
 * Distinct sockets are grouped internally by framework identity. Snapshots
 * expose opaque identifiers because framework hashes may contain credentials.
 * Multiple consumers of one identity share a row until its last owner leaves.
 *
 * Disabled by default — create a registry only when
 * `SvelteRivetKitOptions.connectionInspector` is true.
 *
 * @module
 */

import { createSubscriber } from "svelte/reactivity";
import type { ActorConnStatus } from "rivetkit/client";

/** Public snapshot fields. Anything else on a report is discarded. */
export const CONNECTION_INSPECTOR_SAMPLE_KEYS = [
  "name",
  "key",
  "hash",
  "connStatus",
  "hasConnection",
] as const;

/** One distinct package-managed actor socket. */
export interface ConnectionInspectorSample {
  /** Registry actor name (`counter`, `chat`, …). */
  name: string;
  /** Normalized compound key identifying the actor instance. */
  key: string[];
  /** Opaque inspector identifier, stable while this socket has owners. */
  hash: string;
  /** Last observed connection status. */
  connStatus: ActorConnStatus;
  /** Whether a connection object is currently bound (not necessarily live). */
  hasConnection: boolean;
}

/**
 * Per-handle report written from `applyState`.
 *
 * Extra properties (params, tokens, payloads) are ignored — only the
 * documented fields are copied into the registry.
 */
export interface ConnectionInspectorReport {
  /** Stable id for this `useActor` / `createReactiveActor` handle. */
  ownerId: string;
  name: string;
  key: string | string[];
  hash: string;
  connStatus: ActorConnStatus;
  hasConnection: boolean;
}

/** Live connection registry returned on a RivetKit instance when opted in. */
export interface ConnectionInspector {
  /** Always `true` for a created inspector. Absent / `null` when disabled. */
  readonly enabled: boolean;
  /**
   * Reactive revision. Read inside `$derived` / `$effect` so an overlay
   * can re-snapshot without polling. Bumped on report/unregister only.
   */
  readonly revision: number;
  /** Distinct sockets currently owned by at least one handle. */
  snapshot(): ConnectionInspectorSample[];
  /** Count of distinct sockets whose status is `"connected"`. */
  connectedCount(): number;
  /** Upsert this handle's row. Safe to call from the applyState hot path. */
  report(entry: ConnectionInspectorReport): void;
  /** Drop this handle. The row stays if another handle still owns the hash. */
  unregister(ownerId: string): void;
}

/** Normalize a Rivet actor key to a string array without allocating on arrays. */
export function normalizeActorKey(
  key: string | string[] | undefined,
): string[] {
  if (Array.isArray(key)) return key.map(String);
  if (key == null || key === "") return [];
  return [String(key)];
}

/**
 * Fallback identity when framework-base has not assigned a hash yet.
 * Name + key only — never params.
 */
export function fallbackInspectorHash(name: string, key: string[]): string {
  return JSON.stringify({ name, key });
}

function keysEqual(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

function copySample(
  sample: ConnectionInspectorSample,
): ConnectionInspectorSample {
  return {
    name: sample.name,
    key: sample.key.slice(),
    hash: sample.hash,
    connStatus: sample.connStatus,
    hasConnection: sample.hasConnection,
  };
}

type OwnerRecord = { hash: string };
type HashBucket = {
  owners: Set<string>;
  sample: ConnectionInspectorSample;
};

/**
 * Create an empty live connection registry.
 *
 * Production callers should only construct this when the factory option
 * is explicitly enabled. The default RivetKit path leaves the inspector
 * `null` so applyState stays a no-op.
 */
export function createConnectionInspector(): ConnectionInspector {
  const owners = new Map<string, OwnerRecord>();
  const byHash = new Map<string, HashBucket>();
  let _revision = 0;
  let nextSocketId = 0;
  // Svelte's documented bridge for externally-mutated state:
  // reading `revision` / `snapshot()` inside a `$derived` / `$effect`
  // registers that effect, and `publish()` re-runs it. The registry is
  // in-memory, so `start` only captures `update` for later publishes.
  let publish: (() => void) | undefined;
  const subscribe = createSubscriber((update) => {
    publish = update;
    return () => {
      publish = undefined;
    };
  });

  function bump(): void {
    // Reports arrive from applyState, which can run inside a Svelte
    // effect. The write below contains no reactive reads, so it cannot
    // loop that effect; only effects reading revision/snapshot re-run.
    _revision += 1;
    publish?.();
  }

  function dropOwnerFromHash(hash: string, ownerId: string): void {
    const bucket = byHash.get(hash);
    if (!bucket) return;
    bucket.owners.delete(ownerId);
    if (bucket.owners.size === 0) {
      byHash.delete(hash);
    }
  }

  function report(entry: ConnectionInspectorReport): void {
    const ownerId = entry.ownerId;
    const name = String(entry.name ?? "");
    if (!ownerId || !name) return;

    const key = normalizeActorKey(entry.key);
    const hash = entry.hash || fallbackInspectorHash(name, key);
    const existingBucket = byHash.get(hash);
    // Framework hashes can include params or arbitrary custom-hash input.
    // Preserve grouping internally, but never publish those values.
    const publicHash = existingBucket?.sample.hash ?? `connection:${++nextSocketId}`;
    const next: ConnectionInspectorSample = {
      name,
      key,
      hash: publicHash,
      connStatus: entry.connStatus,
      hasConnection: Boolean(entry.hasConnection),
    };

    const prev = owners.get(ownerId);
    if (prev && prev.hash !== hash) {
      dropOwnerFromHash(prev.hash, ownerId);
    }

    let bucket = byHash.get(hash);
    if (!bucket) {
      bucket = { owners: new Set(), sample: next };
      byHash.set(hash, bucket);
    } else {
      const same =
        bucket.sample.name === next.name &&
        bucket.sample.connStatus === next.connStatus &&
        bucket.sample.hasConnection === next.hasConnection &&
        bucket.sample.hash === next.hash &&
        keysEqual(bucket.sample.key, next.key);
      if (same && bucket.owners.has(ownerId)) {
        owners.set(ownerId, { hash });
        return;
      }
      bucket.sample = next;
    }

    bucket.owners.add(ownerId);
    owners.set(ownerId, { hash });
    bump();
  }

  function unregister(ownerId: string): void {
    const prev = owners.get(ownerId);
    if (!prev) return;
    owners.delete(ownerId);
    dropOwnerFromHash(prev.hash, ownerId);
    bump();
  }

  return {
    enabled: true,
    get revision() {
      subscribe();
      return _revision;
    },
    snapshot() {
      subscribe();
      return [...byHash.values()].map((bucket) => copySample(bucket.sample));
    },
    connectedCount() {
      let n = 0;
      for (const bucket of byHash.values()) {
        if (bucket.sample.connStatus === "connected") n++;
      }
      return n;
    },
    report,
    unregister,
  };
}
