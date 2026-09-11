// Guard for ReactiveActorHandle.reconnect() — the zombie-socket recovery
// primitive (rivetkit.svelte.ts, createReactiveActor's `inner.reconnect`).
//
// THE PROBLEM it solves: a half-open "zombie" WebSocket (NAT/LB idle cull,
// half-open TCP) keeps reporting connStatus === "connected", so a plain
// dispose() + mount() REUSES it — the framework core only creates a fresh
// connection from "idle", and the zombie never reaches "idle". Recovery
// requires an explicit socket replacement.
//
// THE FIX: reconnect() drives framework-base's enabled toggle —
// `getOrCreateActor({enabled:false})` makes framework-base's effect call
// `connection.dispose()` and reset the actor to "idle"; a follow-up
// `getOrCreateActor({enabled:true})` (queued one microtask later, after the
// disable has flushed) re-creates from "idle", opening a brand-new socket.
//
// WHY THIS TEST IS NOT VACUOUS: the discriminating behavior lives entirely
// inside the framework core's effect/idle-gate machinery, so the test
// runs the REAL framework-base (NOT vi.mock'd) over a fake rivetkit client whose
// connections are zombies (report "connected" forever, record dispose()). The
// "control" test proves that WITHOUT reconnect() the zombie persists and no new
// socket opens — so the assertions in the main test can only pass because
// reconnect() actually swapped the socket.

import "./runes-shim.js";
import { describe, expect, test } from "vitest";
import { createRivetKitWithClient } from "../rivetkit.svelte.js";

// Minimal handle surface used by these tests. Casting through this avoids
// `createReactiveActor`'s deep generic instantiation (TS2589, the same
// rivetkit-2.1.10 conditional-type depth the package erases internally) — the
// test exercises runtime behavior, not client types.
interface TestHandle {
  mount: () => () => void;
  reconnect: () => void;
  dispose: () => void;
  onEvent: (event: string, handler: (...args: unknown[]) => void) => () => void;
}
interface TestRivet {
  createReactiveActor: (opts: unknown) => TestHandle;
}
const makeRivet = (client: unknown, opts?: unknown): TestRivet =>
  (
    createRivetKitWithClient as unknown as (
      c: unknown,
      options?: unknown,
    ) => TestRivet
  )(client, opts);

interface FakeConn {
  id: number;
  disposed: boolean;
  onStatusChange: (cb: (status: string) => void) => () => void;
  onError: (cb: (err: Error) => void) => () => void;
  on: (event: string, handler: (...args: unknown[]) => void) => () => void;
  emit: (event: string, ...args: unknown[]) => void;
  dispose: () => void;
}

// A fake rivetkit client that hands out ZOMBIE connections: each one reports
// "connected" immediately and never moves — exactly the half-open socket the
// fix targets. Every connection is recorded so a test can assert which were
// disposed and how many distinct sockets were opened.
function makeZombieClient(): { client: unknown; conns: FakeConn[] } {
  const conns: FakeConn[] = [];

  function makeConn(): FakeConn {
    const listeners = new Map<string, Set<(...args: unknown[]) => void>>();
    const conn: FakeConn = {
      id: conns.length,
      disposed: false,
      // Report "connected" so framework-base sees a live conn — the zombie.
      onStatusChange: (cb) => {
        cb("connected");
        return () => {};
      },
      onError: () => () => {},
      on(event, handler) {
        let set = listeners.get(event);
        if (!set) {
          set = new Set();
          listeners.set(event, set);
        }
        set.add(handler);
        return () => set!.delete(handler);
      },
      emit(event, ...args) {
        listeners.get(event)?.forEach((handler) => handler(...args));
      },
      dispose() {
        this.disposed = true;
      },
    };
    conns.push(conn);
    return conn;
  }

  const handle = { connect: () => makeConn() };
  const client = {
    get: () => handle,
    getOrCreate: () => handle,
  };
  return { client, conns };
}

// Drain the microtask chain (framework-base queues opts-updates + create() on
// queueMicrotask; @tanstack/store flushes setState synchronously). A handful of
// macrotask boundaries deterministically drains it — no real timers/network are
// involved, so this is not flaky.
const flush = async (): Promise<void> => {
  for (let i = 0; i < 5; i++) {
    await new Promise<void>((r) => setTimeout(r, 0));
  }
};

function mountActor() {
  const { client, conns } = makeZombieClient();
  const rivet = makeRivet(client);
  // A compound key identifies this actor instance.
  const handle = rivet.createReactiveActor({
    name: "chat",
    key: ["chat", "agent-1"],
  });
  const unmount = handle.mount();
  return { handle, conns, unmount };
}

describe("ReactiveActorHandle.reconnect()", () => {
  test("disposes the zombie connection and opens a brand-new socket", async () => {
    const { handle, conns } = mountActor();
    await flush();

    // Initial connect produced exactly one (zombie) connection.
    expect(conns.length).toBe(1);
    expect(conns[0].disposed).toBe(false);

    handle.reconnect();
    await flush();

    // The zombie was actually torn down...
    expect(conns[0].disposed).toBe(true);
    // ...and a fresh, distinct connection replaced it.
    expect(conns.length).toBe(2);
    expect(conns[1].disposed).toBe(false);
  });

  test("control: WITHOUT reconnect() the zombie persists — proves the guard is not vacuous", async () => {
    const { conns } = mountActor();
    await flush();
    // Same flushing, no reconnect() call: the zombie is never disposed and no
    // second socket is opened. The only difference from the test above is the
    // reconnect() call — so that call is what causes the dispose + replace.
    await flush();
    expect(conns[0].disposed).toBe(false);
    expect(conns.length).toBe(1);
  });

  test("rebinds onEvent listeners onto the fresh connection", async () => {
    const { handle, conns } = mountActor();
    await flush();

    let calls = 0;
    handle.onEvent("ping", () => {
      calls++;
    });

    handle.reconnect();
    await flush();
    expect(conns.length).toBe(2);

    // The listener registered on the original (now-disposed) connection is
    // re-bound onto the new socket...
    conns[1].emit("ping");
    expect(calls).toBe(1);
    // ...and the old connection no longer drives it.
    conns[0].emit("ping");
    expect(calls).toBe(1);
  });

  test("is a no-op before the actor is ever mounted", () => {
    const { client } = makeZombieClient();
    const rivet = makeRivet(client);
    const handle = rivet.createReactiveActor({
      name: "chat",
      key: ["chat", "agent-1"],
    });
    // Never mounted → nothing to reconnect; must not throw.
    expect(() => handle.reconnect()).not.toThrow();
  });

  test("keeps lifecycle-only enabled out of custom identity hashes", async () => {
    const { client, conns } = makeZombieClient();
    const hashInputs: Array<Record<string, unknown>> = [];
    const rivet = makeRivet(client, {
      hashFunction: (opts: Record<string, unknown>) => {
        hashInputs.push(opts);
        return JSON.stringify({ name: opts.name, key: opts.key });
      },
    });
    const handle = rivet.createReactiveActor({
      name: "chat",
      key: ["chat", "agent-1"],
    });
    handle.mount();
    await flush();

    handle.reconnect();
    await flush();

    expect(conns).toHaveLength(2);
    expect(hashInputs.length).toBeGreaterThanOrEqual(3);
    expect(hashInputs.every((opts) => !("enabled" in opts))).toBe(true);
  });
});


test("remounts after the last owner releases and framework cleanup completes", async () => {
  const { handle, conns, unmount } = mountActor();
  await flush();
  let events = 0;
  handle.onEvent("ping", () => { events++; });
  unmount();
  await flush();
  expect(conns[0].disposed).toBe(true);
  const release = handle.mount();
  await flush();
  expect(conns).toHaveLength(2);
  conns[1].emit("ping");
  conns[0].emit("ping");
  expect(events).toBe(1);
  release();
  handle.dispose();
  await flush();
});


test("retains the shared binding until this handle's final mount releases", async () => {
  const { handle, conns, unmount } = mountActor();
  const releaseSecond = handle.mount();
  await flush();
  let events = 0;
  handle.onEvent("ping", () => { events++; });
  unmount();
  await flush();
  expect(conns).toHaveLength(1);
  expect(conns[0].disposed).toBe(false);
  conns[0].emit("ping");
  expect(events).toBe(1);
  releaseSecond();
  const releaseThird = handle.mount();
  await flush();
  expect(conns).toHaveLength(1);
  expect(conns[0].disposed).toBe(false);
  releaseSecond(); // A stale release cannot release the new mount.
  await flush();
  expect(conns[0].disposed).toBe(false);
  conns[0].emit("ping");
  expect(events).toBe(2);
  releaseThird();
  handle.dispose();
  await flush();
  expect(conns[0].disposed).toBe(true);
});
