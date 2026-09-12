/**
 * Hot-path microbenchmarks for @rivetkit/svelte.
 *
 * Measures the JS-level cost of the most frequently exercised paths:
 *  - Proxy `get` trap (every actor.method access)
 *  - Method cache lookup chain
 *  - applyState (subscribe firing)
 *  - createReactiveActor cold-start
 *  - Action interceptor wrapping
 *  - extract(MaybeGetter)
 *  - mergeActionDefaults
 *  - preloadActor hash generation
 *
 * Reactivity itself is shimmed (runes-shim) — we are measuring the JS
 * overhead the adapter adds on top of Svelte's reactivity primitives.
 *
 * Run with: `bun run --filter @rivetkit/svelte bench`
 */
import "../__tests__/runes-shim.js";
import { bench, describe, vi, beforeAll } from "vitest";

// Benchmarks intentionally mount handles in jsdom-like test context. Suppress
// the development SSR warning so stderr does not dominate benchmark output.
vi.spyOn(console, "warn").mockImplementation(() => {});

// ---------------------------------------------------------------------------
// Framework-base mock — minimal, lifted from existing test mock but tuned
// for benchmark consistency. Every actor shares one mutable state object.
// ---------------------------------------------------------------------------

type MockListener = (...args: unknown[]) => void;

function createMockConnection(id: string) {
  const listeners = new Map<string, Set<MockListener>>();
  return {
    id,
    // Methods return promises because rivetkit actor actions are always async;
    // the interceptor calls `.catch` on the result and would otherwise blow up.
    ping: () => Promise.resolve(`pong:${id}`),
    increment: (n: number) => Promise.resolve(n + 1),
    sendMessage: (payload: unknown) => Promise.resolve(payload),
    on(eventName: string, handler: MockListener) {
      let bucket = listeners.get(eventName);
      if (!bucket) {
        bucket = new Set();
        listeners.set(eventName, bucket);
      }
      bucket.add(handler);
      return () => bucket?.delete(handler);
    },
    emit(eventName: string, ...args: unknown[]) {
      listeners.get(eventName)?.forEach((l) => l(...args));
    },
    connStatus: "connected" as const,
    onStatusChange: () => () => {},
    onError: () => () => {},
    async dispose() {},
  };
}

const baseConnection = createMockConnection("base");
const mockState = {
  connection: baseConnection,
  handle: { id: "handle-base" },
  connStatus: "connected" as const,
  error: null as Error | null,
  hash: "hash-base",
};

const subscribers = new Set<(v: { currentVal: typeof mockState }) => void>();

vi.mock("../internal/framework-base.js", () => ({
  createRivetKit: () => ({
    getOrCreateActor: () => ({
      mount: () => () => {},
      state: {
        get state() {
          return mockState;
        },
        subscribe(cb: (v: { currentVal: typeof mockState }) => void) {
          subscribers.add(cb);
          return () => subscribers.delete(cb);
        },
      },
    }),
  }),
}));

// Imports must come AFTER vi.mock — vi.hoisted guarantees it normally, but
// dynamic import keeps this benchmark file straightforward.
const mod = await import("../rivetkit.svelte.js");
const extractMod = await import("../internal/extract.js");
const { createRivetKitWithClient } = mod;
const { extract } = extractMod;

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

const rivet = createRivetKitWithClient({} as never);
const rivetWithDefaults = createRivetKitWithClient({} as never, {
  actionDefaults: { timeout: 30_000, guardConnection: false },
});
const rivetNoTimeout = createRivetKitWithClient({} as never, {
  // Interceptor with no timeout — isolates the Promise.race/setTimeout cost
  // from the rest of the wrapper.
  actionDefaults: { guardConnection: false },
});
const rivetWithCallbacks = createRivetKitWithClient({} as never, {
  actionDefaults: {
    guardConnection: false,
    onActionStart: () => {},
    onActionSuccess: () => {},
    onActionSettled: () => {},
  },
});

const plainActor = rivet.createReactiveActor({
  name: "chat" as never,
  key: ["bench-plain"],
});
const interceptedActor = rivetWithDefaults.createReactiveActor({
  name: "chat" as never,
  key: ["bench-intercepted"],
});
const noTimeoutActor = rivetNoTimeout.createReactiveActor({
  name: "chat" as never,
  key: ["bench-no-timeout"],
});
const callbackActor = rivetWithCallbacks.createReactiveActor({
  name: "chat" as never,
  key: ["bench-callbacks"],
});

// Many actors for "many simultaneous consumers" benchmarks
const fanOutActors = Array.from({ length: 32 }, (_, i) =>
  rivet.createReactiveActor({
    name: "chat" as never,
    key: [`bench-fan-${i}`],
  }),
);

const mountedActors = [
  plainActor,
  interceptedActor,
  noTimeoutActor,
  callbackActor,
  ...fanOutActors,
];
let applyStateCb: ((v: { currentVal: typeof mockState }) => void) | undefined;

// Push subscribers once so applyState/Proxy methods exercise the connected path
beforeAll(() => {
  for (const actor of mountedActors) actor.mount();
  applyStateCb = [...subscribers][0];
  subscribers.forEach((cb) =>
    cb({ currentVal: { ...mockState, connStatus: "connected" } }),
  );
});

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

describe("proxy.get — known property (state getter)", () => {
  bench("plainActor.connStatus", () => {
    void plainActor.connStatus;
  });
  bench("plainActor.isConnected", () => {
    void plainActor.isConnected;
  });
  bench("interceptedActor.isMutating", () => {
    void interceptedActor.isMutating;
  });
});

describe("proxy.get — unknown property (forwarded actor method)", () => {
  bench("plainActor.ping (cached method)", () => {
    void plainActor.ping;
  });
  bench("interceptedActor.ping (cached, wrapped)", () => {
    void interceptedActor.ping;
  });
  bench("plainActor.ping() call", () => {
    plainActor.ping();
  });
  bench("interceptedActor.ping() call (await)", async () => {
    await interceptedActor.ping();
  });
});

describe("proxy.get — fan-out reads across 32 actors", () => {
  bench("read connStatus on 32 actors", () => {
    for (let i = 0; i < fanOutActors.length; i++) {
      void fanOutActors[i].connStatus;
    }
  });
  bench("read ping on 32 actors", () => {
    for (let i = 0; i < fanOutActors.length; i++) {
      void fanOutActors[i].ping;
    }
  });
});

describe("createReactiveActor construction and mount", () => {
  let i = 0;
  bench("construct handle only", () => {
    rivet.createReactiveActor({
      name: "chat" as never,
      key: [`cold-${i++}`],
    });
  });
  bench("construct handle only, with actionDefaults", () => {
    rivetWithDefaults.createReactiveActor({
      name: "chat" as never,
      key: [`cold-int-${i++}`],
    });
  });
  bench("construct + mount", () => {
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: [`cold-mount-${i++}`],
    });
    const unmount = actor.mount();
    unmount();
    actor.dispose();
  });
  bench("construct + mount, with actionDefaults", () => {
    const actor = rivetWithDefaults.createReactiveActor({
      name: "chat" as never,
      key: [`cold-int-mount-${i++}`],
    });
    const unmount = actor.mount();
    unmount();
    actor.dispose();
  });
  const stableActor = rivet.createReactiveActor({
    name: "chat" as never,
    key: ["stable-mount"],
  });
  bench("mount + unmount existing handle", () => {
    const unmount = stableActor.mount();
    unmount();
  });
});

describe("applyState — subscribe firing", () => {
  // Cast to the subscriber's mockState shape — the rotating variants
  // have `connStatus` literals that widen incompatibly without this.
  const tickStates: Array<typeof mockState> = [
    { ...mockState, connStatus: "connected" } as typeof mockState,
    {
      ...mockState,
      connStatus: "disconnected" as never,
      error: new Error("x"),
    } as typeof mockState,
    { ...mockState, connStatus: "reconnecting" as never } as typeof mockState,
    {
      ...mockState,
      connStatus: "connected",
      error: null,
    } as typeof mockState,
  ];
  let idx = 0;
  bench("subscribe push (rotating statuses)", () => {
    applyStateCb?.({ currentVal: tickStates[idx++ & 3]! });
  });
  bench("subscribe push (identical value)", () => {
    applyStateCb?.({ currentVal: tickStates[0]! });
  });
});

describe("extract(MaybeGetter)", () => {
  const direct = { name: "chat", key: ["k"] };
  const getter = () => ({ name: "chat", key: ["k"] });
  bench("static value", () => {
    extract(direct);
  });
  bench("getter thunk", () => {
    extract(getter);
  });
});

describe("preloadActor hash", () => {
  // Direct probe of the JSON.stringify approach the source uses.
  function lengthPrefixedHash(name: string, keyArray: string[]) {
    let hash = `${name.length}:${name}`;
    for (const part of keyArray) hash += `|${part.length}:${part}`;
    return hash;
  }

  bench("JSON.stringify hash (string key)", () => {
    JSON.stringify(["doc", ["abc-123"], null]);
  });
  bench("JSON.stringify hash (compound key)", () => {
    JSON.stringify(["doc", ["org-1", "user-2", "thread-3"], null]);
  });
  bench("length-prefixed hash (string key)", () => {
    lengthPrefixedHash("doc", ["abc-123"]);
  });
  bench("length-prefixed hash (compound key)", () => {
    lengthPrefixedHash("doc", ["org-1", "user-2", "thread-3"]);
  });
  bench("manual hash (string key)", () => {
    "doc\x00abc-123";
  });
});

describe("mergeActionDefaults equivalent", () => {
  const a = { timeout: 30_000, throwOnError: false as boolean };
  const b = { timeout: 60_000 };
  bench("shallow spread", () => {
    void { ...a, ...b };
  });
});

describe("interceptedActor — concurrent actions", () => {
  bench("Promise.all(8 × ping)", async () => {
    await Promise.all([
      interceptedActor.ping(),
      interceptedActor.ping(),
      interceptedActor.ping(),
      interceptedActor.ping(),
      interceptedActor.ping(),
      interceptedActor.ping(),
      interceptedActor.ping(),
      interceptedActor.ping(),
    ]);
  });
});

describe("whenConnected (immediate-resolve path)", () => {
  bench("already-connected fast path", async () => {
    await plainActor.whenConnected();
  });
});

// Floor benchmarks: establish the minimum cost of `await Promise.resolve`
// at the engine level so we can subtract that from the interceptor numbers
// and see how much overhead is intrinsic to async/await vs added by us.
describe("async/await floor", () => {
  bench("bare await Promise.resolve()", async () => {
    await Promise.resolve(42);
  });
  bench("await baseConnection.ping() (no proxy)", async () => {
    await baseConnection.ping();
  });
  bench("await plainActor.ping() (proxy, no interceptor)", async () => {
    await plainActor.ping();
  });
  bench("await noTimeoutActor.ping() (interceptor, no timeout)", async () => {
    await noTimeoutActor.ping();
  });
  bench("await callbackActor.ping() (interceptor + callbacks)", async () => {
    await callbackActor.ping();
  });
});
