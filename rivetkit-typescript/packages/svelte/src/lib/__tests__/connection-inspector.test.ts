/**
 * Pins the opt-in connection inspector: per-handle ownership, hash
 * deduplication, status replacement, privacy (no params), and the
 * factory wiring through createReactiveActor applyState / dispose.
 */
import "./runes-shim.js";
import { describe, expect, test, vi, beforeEach } from "vitest";
import type { ActorConnStatus } from "rivetkit/client";
import {
  CONNECTION_INSPECTOR_SAMPLE_KEYS,
  createConnectionInspector,
  fallbackInspectorHash,
  normalizeActorKey,
} from "../connection-inspector.svelte.js";

const frameworkMock = vi.hoisted(() => {
  type Subscriber = (value: { currentVal: MockActorState }) => void;
  type MockActorState = {
    connection: { id: string } | null;
    handle: { id: string };
    connStatus: ActorConnStatus;
    error: Error | null;
    hash: string;
  };

  const subscribers = new Set<Subscriber>();
  const statesByHash = new Map<string, MockActorState>();

  function identityHash(opts: { name?: string; key?: string | string[] }): string {
    return JSON.stringify({ name: opts.name, key: opts.key });
  }

  function makeState(hash: string): MockActorState {
    return {
      connection: { id: `conn:${hash}` },
      handle: { id: `handle:${hash}` },
      connStatus: "idle",
      error: null,
      hash,
    };
  }

  const getOrCreateActor = vi.fn((actorOpts: { name?: string; key?: string | string[] }) => {
    const hash = identityHash(actorOpts);
    let current = statesByHash.get(hash);
    if (!current) {
      current = makeState(hash);
      statesByHash.set(hash, current);
    }

    return {
      mount: vi.fn(() => vi.fn()),
      state: {
        get state() {
          return statesByHash.get(hash) ?? current;
        },
        subscribe(callback: Subscriber) {
          subscribers.add(callback);
          return () => subscribers.delete(callback);
        },
      },
    };
  });

  function push(hash: string, next: Partial<MockActorState>) {
    const current = statesByHash.get(hash) ?? makeState(hash);
    const updated = { ...current, ...next, hash };
    statesByHash.set(hash, updated);
    subscribers.forEach((subscriber) => subscriber({ currentVal: updated }));
  }

  function reset() {
    subscribers.clear();
    statesByHash.clear();
    getOrCreateActor.mockClear();
  }

  return { getOrCreateActor, push, identityHash, reset };
});

vi.mock("../internal/framework-base.js", () => ({
  createRivetKit: vi.fn(() => ({
    getOrCreateActor: frameworkMock.getOrCreateActor,
  })),
}));

vi.mock("esm-env", () => ({
  BROWSER: true,
  DEV: false,
}));

import { createRivetKitWithClient } from "../rivetkit.svelte.js";

describe("normalizeActorKey / fallbackInspectorHash", () => {
  test("normalizes a string key and leaves empty keys as an empty array", () => {
    expect(normalizeActorKey("room-1")).toEqual(["room-1"]);
    expect(normalizeActorKey(["org", "page"])).toEqual(["org", "page"]);
    expect(normalizeActorKey(undefined)).toEqual([]);
    expect(normalizeActorKey("")).toEqual([]);
  });

  test("fallback hash is name + key only", () => {
    expect(fallbackInspectorHash("page", ["p1"])).toBe(
      JSON.stringify({ name: "page", key: ["p1"] }),
    );
  });
});

describe("createConnectionInspector", () => {
  test("reports a sample, updates status, and counts connected sockets", () => {
    const inspector = createConnectionInspector();
    expect(inspector.enabled).toBe(true);
    expect(inspector.snapshot()).toEqual([]);

    inspector.report({
      ownerId: "h1",
      name: "page",
      key: ["page-1"],
      hash: "hash-page",
      connStatus: "connecting",
      hasConnection: false,
    });

    expect(inspector.snapshot()).toEqual([
      {
        name: "page",
        key: ["page-1"],
        hash: expect.any(String),
        connStatus: "connecting",
        hasConnection: false,
      },
    ]);
    expect(inspector.connectedCount()).toBe(0);

    const revisionAfterRegister = inspector.revision;
    inspector.report({
      ownerId: "h1",
      name: "page",
      key: ["page-1"],
      hash: "hash-page",
      connStatus: "connected",
      hasConnection: true,
    });

    expect(inspector.revision).toBeGreaterThan(revisionAfterRegister);
    expect(inspector.connectedCount()).toBe(1);
    expect(inspector.snapshot()[0]?.connStatus).toBe("connected");
  });

  test("deduplicates shared consumers by hash and keeps the row until the last owner unregisters", () => {
    const inspector = createConnectionInspector();
    inspector.report({
      ownerId: "a",
      name: "chat",
      key: ["ws-1"],
      hash: "hash-ws",
      connStatus: "connected",
      hasConnection: true,
    });
    inspector.report({
      ownerId: "b",
      name: "chat",
      key: ["ws-1"],
      hash: "hash-ws",
      connStatus: "connected",
      hasConnection: true,
    });

    expect(inspector.snapshot()).toHaveLength(1);
    expect(inspector.connectedCount()).toBe(1);

    inspector.unregister("a");
    expect(inspector.snapshot()).toHaveLength(1);
    expect(inspector.snapshot()[0]?.name).toBe("chat");

    inspector.unregister("b");
    expect(inspector.snapshot()).toEqual([]);
    expect(inspector.connectedCount()).toBe(0);
  });

  test("moving a handle to a new hash drops the old row when it has no remaining owners", () => {
    const inspector = createConnectionInspector();
    inspector.report({
      ownerId: "h1",
      name: "page",
      key: ["old"],
      hash: "hash-old",
      connStatus: "connected",
      hasConnection: true,
    });
    inspector.report({
      ownerId: "h1",
      name: "page",
      key: ["new"],
      hash: "hash-new",
      connStatus: "connecting",
      hasConnection: false,
    });

    const hashes = inspector.snapshot().map((row) => row.hash).sort();
    expect(hashes).toHaveLength(1);
    expect(hashes[0]).not.toBe("hash-new");
    expect(inspector.snapshot()[0]?.key).toEqual(["new"]);
  });

  test("discards params, tokens, and any extra report fields from the snapshot", () => {
    const inspector = createConnectionInspector();
    inspector.report({
      ownerId: "h1",
      name: "user",
      key: ["user@example.com"],
      hash: "hash-user",
      connStatus: "connected",
      hasConnection: true,
      params: { authToken: "secret-jwt", token: "also-secret" },
      getParams: () => ({ authToken: "secret-jwt" }),
      payload: { email: "user@example.com" },
    } as never);

    const [row] = inspector.snapshot();
    expect(row).toBeDefined();
    expect(Object.keys(row!).sort()).toEqual(
      [...CONNECTION_INSPECTOR_SAMPLE_KEYS].sort(),
    );
    expect(JSON.stringify(row)).not.toContain("secret");
    expect(JSON.stringify(row)).not.toContain("authToken");
    expect(JSON.stringify(row)).not.toContain("params");
  });

  test("unregister is a no-op for an unknown owner", () => {
    const inspector = createConnectionInspector();
    inspector.unregister("missing");
    expect(inspector.snapshot()).toEqual([]);
    expect(inspector.revision).toBe(0);
  });
});

describe("createRivetKitWithClient connectionInspector option", () => {
  beforeEach(() => {
    frameworkMock.reset();
  });

  test("leaves the inspector null when the option is omitted", () => {
    const rivet = createRivetKitWithClient({} as never);
    expect(rivet.connectionInspector).toBeNull();

    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();
    expect(rivet.connectionInspector).toBeNull();
    actor.dispose();
  });

  test("registers, updates, and removes a reactive actor through applyState", () => {
    const rivet = createRivetKitWithClient({} as never, {
      connectionInspector: true,
    });
    const inspector = rivet.connectionInspector;
    expect(inspector?.enabled).toBe(true);

    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    expect(inspector?.snapshot()).toEqual([]);

    actor.mount();
    const hash = frameworkMock.identityHash({
      name: "chat",
      key: ["room-1"],
    });
    expect(inspector?.snapshot()).toEqual([
      {
        name: "chat",
        key: ["room-1"],
        hash: expect.any(String),
        connStatus: "idle",
        hasConnection: true,
      },
    ]);

    frameworkMock.push(hash, { connStatus: "connected" });
    expect(inspector?.connectedCount()).toBe(1);
    expect(inspector?.snapshot()[0]?.connStatus).toBe("connected");

    actor.dispose();
    expect(inspector?.snapshot()).toEqual([]);
  });

  test("keeps one row when two handles share an identity and survive a single dispose", () => {
    const rivet = createRivetKitWithClient({} as never, {
      connectionInspector: true,
    });
    const inspector = rivet.connectionInspector;

    const a = rivet.createReactiveActor({
      name: "page" as never,
      key: ["p1"],
    });
    const b = rivet.createReactiveActor({
      name: "page" as never,
      key: ["p1"],
    });
    a.mount();
    b.mount();

    expect(inspector?.snapshot()).toHaveLength(1);
    a.dispose();
    expect(inspector?.snapshot()).toHaveLength(1);
    expect(inspector?.snapshot()[0]?.name).toBe("page");
    b.dispose();
    expect(inspector?.snapshot()).toEqual([]);
  });

  test("unmount without dispose drops the row so leaked handles do not linger", () => {
    const rivet = createRivetKitWithClient({} as never, {
      connectionInspector: true,
    });
    const actor = rivet.createReactiveActor({
      name: "tile" as never,
      key: ["t1"],
    });
    const release = actor.mount();
    expect(rivet.connectionInspector?.snapshot()).toHaveLength(1);
    release();
    expect(rivet.connectionInspector?.snapshot()).toEqual([]);
  });
});


test("redacts credential-bearing framework hashes without merging distinct sockets", () => {
  const inspector = createConnectionInspector();
  const report = (ownerId: string, token: string) => inspector.report({
    ownerId, name: "counter", key: ["same"],
    hash: JSON.stringify({ name: "counter", key: ["same"], params: { token } }),
    connStatus: "connected", hasConnection: true,
  });
  report("a", "secret-one");
  report("b", "secret-one");
  report("c", "secret-two");
  const rows = inspector.snapshot();
  expect(rows).toHaveLength(2);
  expect(new Set(rows.map(row => row.hash)).size).toBe(2);
  expect(JSON.stringify(rows)).not.toContain("secret");
  expect(JSON.stringify(rows)).not.toContain("params");
  report("a", "secret-one");
  expect(inspector.snapshot()).toEqual(rows);
  inspector.unregister("a");
  expect(inspector.snapshot()).toEqual(rows);
  inspector.unregister("b");
  expect(inspector.snapshot()).toEqual([rows[1]]);
});
