import "./runes-shim.js";
import { describe, expect, test, vi, beforeEach } from "vitest";
import type { ActorConnStatus } from "rivetkit/client";

const frameworkMock = vi.hoisted(() => {
  type Listener = (...args: unknown[]) => void;
  type Subscriber = (value: { currentVal: MockActorState }) => void;
  type MockConnection = {
    id: string;
    ping: () => string;
    admin: { ping: () => string };
    on: (eventName: string, handler: Listener) => () => void;
    emit: (eventName: string, ...args: unknown[]) => void;
  };
  type MockActorState = {
    connection: MockConnection;
    handle: { id: string };
    connStatus: ActorConnStatus;
    error: Error | null;
    hash: string;
  };

  const subscribers = new Set<Subscriber>();

  function createConnection(id: string): MockConnection {
    const listeners = new Map<string, Set<Listener>>();

    return {
      id,
      ping: () => `pong:${id}`,
      admin: { ping: () => `admin-pong:${id}` },
      on(eventName: string, handler: Listener) {
        let eventListeners = listeners.get(eventName);
        if (!eventListeners) {
          eventListeners = new Set();
          listeners.set(eventName, eventListeners);
        }

        eventListeners.add(handler);
        return () => eventListeners?.delete(handler);
      },
      emit(eventName: string, ...args: unknown[]) {
        listeners.get(eventName)?.forEach((listener) => listener(...args));
      },
    };
  }

  let currentState: MockActorState;
  let lastMount: ReturnType<typeof vi.fn> | null = null;
  let lastUnmount: ReturnType<typeof vi.fn> | null = null;

  const getOrCreateActor = vi.fn(() => {
    lastUnmount = vi.fn();
    lastMount = vi.fn(() => lastUnmount!);
    return {
      mount: lastMount,
      state: {
        get state() {
          return currentState;
        },
        subscribe(callback: Subscriber) {
          subscribers.add(callback);
          return () => subscribers.delete(callback);
        },
      },
    };
  });

  function push(next: Partial<MockActorState>) {
    currentState = { ...currentState, ...next };
    subscribers.forEach((subscriber) =>
      subscriber({ currentVal: currentState }),
    );
  }

  function reset() {
    subscribers.clear();
    currentState = {
      connection: createConnection("one"),
      handle: { id: "handle-one" },
      connStatus: "idle",
      error: null,
      hash: "hash-one",
    };
    getOrCreateActor.mockClear();
    lastMount = null;
    lastUnmount = null;
  }

  reset();

  return {
    getOrCreateActor,
    currentState: () => currentState,
    push,
    replaceConnection(id: string) {
      const connection = createConnection(id);
      push({
        connection,
        handle: { id: `handle-${id}` },
        hash: `hash-${id}`,
      });
      return connection;
    },
    lastMount: () => lastMount,
    lastUnmount: () => lastUnmount,
    reset,
  };
});

vi.mock("../internal/framework-base.js", () => ({
  createRivetKit: vi.fn(() => ({
    getOrCreateActor: frameworkMock.getOrCreateActor,
  })),
}));

// preConnect() short-circuits to an inert handle under SSR (BROWSER=false).
// Pin BROWSER=true so it actually mounts; createReactiveActor's own tests are
// unaffected (they only branch on BROWSER for a dev-time SSR warning).
vi.mock("esm-env", () => ({
  BROWSER: true,
  DEV: false,
}));

import { createRivetKitWithClient } from "../rivetkit.svelte.js";

describe("createReactiveActor", () => {
  beforeEach(() => {
    frameworkMock.reset();
  });

  test("defers framework subscription until mount", () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });

    expect(frameworkMock.getOrCreateActor).not.toHaveBeenCalled();

    const unmount = actor.mount();
    expect(frameworkMock.getOrCreateActor).toHaveBeenCalledTimes(1);
    expect(frameworkMock.lastMount()).toHaveBeenCalledTimes(1);

    unmount();
  });

  test("keeps proxied actor methods stable across connection changes", () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();

    const firstPing = actor.ping;
    const secondPing = actor.ping;

    expect(firstPing).toBe(secondPing);
    expect(firstPing()).toBe("pong:one");

    frameworkMock.replaceConnection("two");

    const thirdPing = actor.ping;
    expect(thirdPing).toBe(firstPing);
    expect(firstPing()).toBe("pong:two");
    expect(thirdPing()).toBe("pong:two");
  });

  test("supports handlers captured before mount and nested Rivet actions", () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    const ping = actor.ping;
    // This runtime-only test intentionally uses an erased registry. Describe
    // the nested fake action locally now that the public factory no longer
    // leaks `any` into consumers.
    const adminPing = (actor as unknown as { admin: { ping: () => string } })
      .admin.ping;

    actor.mount();

    expect(ping()).toBe("pong:one");
    expect(adminPing()).toBe("admin-pong:one");

    frameworkMock.replaceConnection("two");
    expect(adminPing()).toBe("admin-pong:two");
  });

  test("is not Promise-like", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();

    await expect(Promise.resolve(actor)).resolves.toBe(actor);
  });

  test("detaches captured handlers from the connection on dispose", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();
    const ping = actor.ping;

    actor.dispose();

    expect(actor.connection).toBeNull();
    expect(actor.connStatus).toBe("idle");
    await expect(ping()).rejects.toMatchObject({
      code: "ACTOR_NOT_YET_CONNECTED",
      connStatus: "idle",
    });
  });

  test("preserves lastError and tracks hasEverConnected", () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();

    expect(actor.lastError).toBe(null);
    expect(actor.hasEverConnected).toBe(false);

    frameworkMock.push({
      connStatus: "disconnected",
      error: new Error("boom"),
    });

    expect(actor.error?.message).toBe("boom");
    expect(actor.lastError?.message).toBe("boom");
    expect(actor.hasEverConnected).toBe(false);

    frameworkMock.push({
      connStatus: "connected",
      error: null,
    });

    expect(actor.isConnected).toBe(true);
    expect(actor.hasEverConnected).toBe(true);
    expect(actor.lastError?.message).toBe("boom");

    frameworkMock.push({
      connStatus: "disconnected",
      error: null,
    });

    expect(actor.error).toBe(null);
    expect(actor.lastError?.message).toBe("boom");
  });

  test("whenConnected resolves immediately when already connected", async () => {
    frameworkMock.push({ connStatus: "connected" });
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();

    const result = await actor.whenConnected();
    expect(result).toBe(true);
  });

  test("whenConnected resolves when connection is established", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();

    expect(actor.isConnected).toBe(false);

    const promise = actor.whenConnected(5_000);

    // Simulate connection after a tick
    frameworkMock.push({ connStatus: "connected" });

    const result = await promise;
    expect(result).toBe(true);
  });

  test("whenConnected resolves false on timeout", async () => {
    vi.useFakeTimers();

    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();

    const promise = actor.whenConnected(100);

    // Advance past timeout without connecting
    vi.advanceTimersByTime(150);

    const result = await promise;
    expect(result).toBe(false);

    vi.useRealTimers();
  });

  test("dispose cancels pending whenConnected with false", async () => {
    vi.useFakeTimers();

    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();

    const promise = actor.whenConnected(30_000);

    // Dispose before connection is established
    actor.dispose();

    const result = await promise;
    expect(result).toBe(false);

    vi.useRealTimers();
  });

  test("rebinds event listeners when the connection changes", () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();

    const firstConnection = frameworkMock.currentState().connection;
    const received: string[] = [];

    actor.onEvent("message", (payload: unknown) => {
      received.push(String(payload));
    });

    firstConnection.emit("message", "one");
    expect(received).toEqual(["one"]);

    const secondConnection = frameworkMock.replaceConnection("two");

    firstConnection.emit("message", "stale");
    secondConnection.emit("message", "two");

    expect(received).toEqual(["one", "two"]);
  });

  test("dispose unmounts active framework refs", () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });

    actor.mount();
    actor.dispose();

    expect(frameworkMock.lastUnmount()).toHaveBeenCalledTimes(1);
  });

  test("binds event listeners registered before mount", () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    const received: string[] = [];

    actor.onEvent("message", (payload: unknown) => {
      received.push(String(payload));
    });

    actor.mount();
    frameworkMock.currentState().connection.emit("message", "mounted");

    expect(received).toEqual(["mounted"]);
  });
});

describe("preConnect", () => {
  beforeEach(() => {
    frameworkMock.reset();
  });

  test("opens a connection eagerly and disposes on demand", async () => {
    const rivet = createRivetKitWithClient({} as never);

    const handle = rivet.preConnect({ name: "chat" as never, key: ["room-1"] });

    // Unlike createReactiveActor (which defers until mount), preConnect mounts
    // immediately so the socket is live before any component takes over.
    expect(frameworkMock.getOrCreateActor).toHaveBeenCalledTimes(1);
    expect(frameworkMock.lastMount()).toHaveBeenCalledTimes(1);

    await handle.dispose();
    expect(frameworkMock.lastUnmount()).toHaveBeenCalledTimes(1);
  });

  test("dispose is idempotent", async () => {
    const rivet = createRivetKitWithClient({} as never);

    const handle = rivet.preConnect({ name: "chat" as never, key: ["room-1"] });
    await handle.dispose();
    await handle.dispose();

    expect(frameworkMock.lastUnmount()).toHaveBeenCalledTimes(1);
  });

  test("forces enabled for an explicit eager connection", async () => {
    const rivet = createRivetKitWithClient({} as never);

    const handle = rivet.preConnect({
      name: "chat" as never,
      key: ["room-1"],
      enabled: false,
    });

    expect(frameworkMock.getOrCreateActor).toHaveBeenCalledWith(
      expect.objectContaining({ enabled: true }),
    );
    await handle.dispose();
  });
});
