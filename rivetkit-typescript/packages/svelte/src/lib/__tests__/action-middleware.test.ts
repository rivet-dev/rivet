import "./runes-shim.js";
import { afterEach, describe, expect, test, vi, beforeEach } from "vitest";
import type { ActorConnStatus } from "rivetkit/client";
import { rerunEffects, resetEffects } from "./runes-shim.js";

// ---------------------------------------------------------------------------
// Mock — identical shape to reactive-actor.test.ts, but with async actions
// ---------------------------------------------------------------------------

const frameworkMock = vi.hoisted(() => {
  type Listener = (...args: unknown[]) => void;
  type Subscriber = (value: { currentVal: MockActorState }) => void;
  type MockConnection = {
    id: string;
    ping: () => string;
    increment: (amount: number) => Promise<number>;
    failAction: () => Promise<never>;
    slowAction: () => Promise<string>;
    syncValue: () => number;
    syncThrow: () => never;
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
  type HashFunction = (opts: Record<string, unknown>) => string;

  const subscribers = new Set<Subscriber>();
  const defaultHash: HashFunction = ({ name, key, params, noCreate }) =>
    JSON.stringify({ name, key, params, noCreate });
  let hashFunction: HashFunction = defaultHash;

  function createConnection(id: string): MockConnection {
    const listeners = new Map<string, Set<Listener>>();

    return {
      id,
      ping: () => `pong:${id}`,
      increment: vi.fn(async (amount: number) => amount + 1),
      failAction: vi.fn(async () => {
        throw new Error("action failed");
      }),
      slowAction: vi.fn(
        () =>
          new Promise<string>((resolve) =>
            setTimeout(() => resolve("done"), 5_000),
          ),
      ),
      syncValue: vi.fn(() => 42),
      syncThrow: vi.fn(() => {
        throw new Error("sync action failed");
      }),
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

  const getOrCreateActor = vi.fn((actorOpts: Record<string, unknown>) => {
    const normalizedOpts = {
      ...actorOpts,
      enabled: actorOpts.enabled ?? true,
    };
    return {
      key: hashFunction(normalizedOpts),
      mount: vi.fn(() => vi.fn()),
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

  function configure(opts?: { hashFunction?: HashFunction }): void {
    hashFunction = opts?.hashFunction ?? defaultHash;
  }

  function push(next: Partial<MockActorState>): void {
    currentState = { ...currentState, ...next };
    subscribers.forEach((subscriber) =>
      subscriber({ currentVal: currentState }),
    );
  }

  function reset(): void {
    subscribers.clear();
    hashFunction = defaultHash;
    currentState = {
      connection: createConnection("one"),
      handle: { id: "handle-one" },
      connStatus: "connected",
      error: null,
      hash: "hash-one",
    };
    getOrCreateActor.mockClear();
  }

  reset();

  return {
    getOrCreateActor,
    currentState: () => currentState,
    push,
    reset,
    createConnection,
    configure,
  };
});

vi.mock("../internal/framework-base.js", () => ({
  createRivetKit: vi.fn((_client, opts) => {
    frameworkMock.configure(opts);
    return { getOrCreateActor: frameworkMock.getOrCreateActor };
  }),
}));

import { createRivetKitWithClient } from "../rivetkit.svelte.js";

// ---------------------------------------------------------------------------
// Tests — action middleware via actionDefaults
// ---------------------------------------------------------------------------

describe("action middleware (createReactiveActor)", () => {
  beforeEach(() => {
    resetEffects();
    frameworkMock.reset();
    vi.useFakeTimers();
  });

  afterEach(() => {
    resetEffects();
    vi.useRealTimers();
  });

  test("without actionDefaults, actions are plain pass-through (no tracking)", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
    });
    actor.mount();

    // Action tracking state has defaults but no interceptor
    expect(actor.isMutating).toBe(false);
    expect(actor.pendingActions).toBe(0);
    expect(actor.lastActionError).toBe(null);
    expect(actor.lastAction).toBe(null);

    // Actions pass through directly — no interception
    const result = await actor.increment(5);
    expect(result).toBe(6);

    // No tracking occurred (no actionDefaults configured)
    expect(actor.isMutating).toBe(false);
    expect(actor.lastAction).toBe(null);
  });

  test("with actionDefaults, tracks isMutating and pendingActions", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {},
    });
    actor.mount();

    expect(actor.isMutating).toBe(false);
    expect(actor.pendingActions).toBe(0);

    const promise = actor.increment(5);
    // Synchronously after calling, state is updated
    expect(actor.isMutating).toBe(true);
    expect(actor.pendingActions).toBe(1);
    expect(actor.lastAction).toBe("increment");

    await promise;

    expect(actor.isMutating).toBe(false);
    expect(actor.pendingActions).toBe(0);
  });

  test("captures errors to lastActionError (throwOnError: false default)", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {},
    });
    actor.mount();

    // Call an action that throws
    const result = await actor.failAction();

    // Error captured reactively, not thrown
    expect(result).toBeUndefined();
    expect(actor.lastActionError).toBeInstanceOf(Error);
    expect(actor.lastActionError?.message).toBe("action failed");
    expect(actor.lastAction).toBe("failAction");
    expect(actor.isMutating).toBe(false);
  });

  test("captures synchronous action throws and clears pending state", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {},
    });
    actor.mount();

    const result = await actor.syncThrow();

    expect(result).toBeUndefined();
    expect(actor.lastActionError?.message).toBe("sync action failed");
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
  });

  test("supports synchronous non-Promise action results", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {},
    });
    actor.mount();

    const result = await actor.syncValue();

    expect(result).toBe(42);
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
  });

  test("clears lastActionError on next successful action", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {},
    });
    actor.mount();

    await actor.failAction();
    expect(actor.lastActionError).not.toBe(null);

    await actor.increment(1);
    expect(actor.lastActionError).toBe(null);
  });

  test("throwOnError: true re-throws the error", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: { throwOnError: true },
    });
    actor.mount();

    await expect(actor.failAction()).rejects.toThrow("action failed");
    // Error is still captured reactively even when thrown
    expect(actor.lastActionError?.message).toBe("action failed");
  });

  test("throwOnError as function — called per error to decide", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {
        throwOnError: (_err: Error, actionName: string) =>
          actionName === "failAction",
      },
    });
    actor.mount();

    // failAction should throw (function returns true for it)
    await expect(actor.failAction()).rejects.toThrow("action failed");
  });

  test("timeout causes action to fail", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: { timeout: 100 },
    });
    actor.mount();

    const promise = actor.slowAction();

    // Advance past the timeout
    vi.advanceTimersByTime(150);

    const result = await promise;
    expect(result).toBeUndefined();
    expect(actor.lastActionError?.message).toContain("timed out");
    expect(actor.isMutating).toBe(false);
  });

  test("synchronous disposal during non-abort-aware dispatch settles without waiting for timeout", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["sync-dispose"],
      actionDefaults: { timeout: 3_600_000 },
    });
    actor.mount();
    frameworkMock.currentState().connection.slowAction = vi.fn(() => {
      actor.dispose();
      return new Promise<string>(() => {});
    });
    await expect(actor.slowAction()).resolves.toBeUndefined();
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
  });

  test("per-action read deadline settles counters without lowering mutation timeout", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-read-policy"],
      actionDefaults: {
        timeout: 3_600_000,
        timeoutByAction: { slowAction: 15 },
      },
    });
    actor.mount();
    const read = actor.slowAction();
    expect(actor.pendingActions).toBe(1);
    await vi.advanceTimersByTimeAsync(20);
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
    await expect(read).resolves.toBeUndefined();
    expect(actor.lastActionError?.message).toContain("15ms");
  });

  test("read deadline and disposal abort the raw SDK action with its correct receiver", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["abort-read"],
      actionDefaults: {
        timeout: 3_600_000,
        timeoutByAction: { slowAction: 15 },
      },
    });
    actor.mount();
    const conn = frameworkMock.currentState().connection;
    const signals: AbortSignal[] = [];
    Object.assign(conn, {
      action(
        this: unknown,
        opts: { name: string; args: unknown[]; signal: AbortSignal },
      ) {
        expect(this).toBe(conn);
        signals.push(opts.signal);
        return new Promise((_, reject) =>
          opts.signal.addEventListener(
            "abort",
            () => reject(new Error("SDK aborted")),
            { once: true },
          ),
        );
      },
    });
    const read = actor.slowAction();
    await vi.advanceTimersByTimeAsync(20);
    await expect(read).resolves.toBeUndefined();
    expect(signals[0]!.aborted).toBe(true);
    expect(actor.pendingActions).toBe(0);
    const mutation = actor.increment(1);
    await vi.advanceTimersByTimeAsync(20);
    expect(actor.pendingActions).toBe(1);
    expect(signals[1]!.aborted).toBe(false);
    actor.dispose();
    await expect(mutation).resolves.toBeUndefined();
    expect(signals[1]!.aborted).toBe(true);
    expect(actor.pendingActions).toBe(0);
  });

  test.each([0, -1, Number.NaN, Number.POSITIVE_INFINITY])(
    "invalid named timeout %s falls back to default",
    async (timeout) => {
      const rivet = createRivetKitWithClient({} as never);
      const actor = rivet.createReactiveActor({
        name: "chat" as never,
        key: ["invalid-timeout"],
        actionDefaults: {
          timeout: 20,
          timeoutByAction: { slowAction: timeout },
        },
      });
      actor.mount();
      const read = actor.slowAction();
      await vi.advanceTimersByTimeAsync(5);
      expect(actor.pendingActions).toBe(1);
      await vi.advanceTimersByTimeAsync(20);
      await expect(read).resolves.toBeUndefined();
      expect(actor.pendingActions).toBe(0);
    },
  );

  test("uses one timeout deadline across connection wait and dispatch", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: { timeout: 1_000 },
    });
    actor.mount();

    frameworkMock.push({ connStatus: "connecting" });
    const pending = actor.slowAction();

    await vi.advanceTimersByTimeAsync(600);
    frameworkMock.push({ connStatus: "connected" });
    await vi.advanceTimersByTimeAsync(0);
    expect(
      frameworkMock.currentState().connection.slowAction,
    ).toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(399);
    expect(actor.isMutating).toBe(true);
    await vi.advanceTimersByTimeAsync(1);

    await expect(pending).resolves.toBeUndefined();
    expect(actor.lastActionError?.message).toContain("timed out after 1000ms");
    expect(actor.pendingActions).toBe(0);
  });

  test("resetActionState clears error and lastAction", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {},
    });
    actor.mount();

    await actor.failAction();
    expect(actor.lastActionError).not.toBe(null);
    expect(actor.lastAction).toBe("failAction");

    actor.resetActionState();
    expect(actor.lastActionError).toBe(null);
    expect(actor.lastAction).toBe(null);
  });

  test("lifecycle callbacks fire in order", async () => {
    const log: string[] = [];
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {
        onActionStart: (name: string) => log.push(`start:${name}`),
        onActionSuccess: (name: string) => log.push(`success:${name}`),
        onActionError: (_err: Error, name: string) => log.push(`error:${name}`),
        onActionSettled: (name: string) => log.push(`settled:${name}`),
      },
    });
    actor.mount();

    await actor.increment(5);
    expect(log).toEqual([
      "start:increment",
      "success:increment",
      "settled:increment",
    ]);

    log.length = 0;
    await actor.failAction();
    expect(log).toEqual([
      "start:failAction",
      "error:failAction",
      "settled:failAction",
    ]);
  });

  test("cleans up tracking when onActionStart throws", async () => {
    const onSettled = vi.fn();
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {
        onActionStart: () => {
          throw new Error("start callback failed");
        },
        onActionSettled: onSettled,
      },
    });
    actor.mount();

    await expect(actor.increment(5)).rejects.toThrow("start callback failed");
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
    expect(onSettled).toHaveBeenCalledWith("increment");
  });

  test("cleans up tracking and settles when success callbacks throw", async () => {
    const onSettled = vi.fn();
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {
        onActionSuccess: () => {
          throw new Error("success callback failed");
        },
        onActionSettled: onSettled,
      },
    });
    actor.mount();

    await expect(actor.increment(5)).rejects.toThrow("success callback failed");
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
    expect(onSettled).toHaveBeenCalledWith("increment");
  });

  test("cleans up tracking and settles when error callbacks throw", async () => {
    const onSettled = vi.fn();
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {
        onActionError: () => {
          throw new Error("error callback failed");
        },
        onActionSettled: onSettled,
      },
    });
    actor.mount();

    await expect(actor.failAction()).rejects.toThrow("error callback failed");
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
    expect(onSettled).toHaveBeenCalledWith("failAction");
  });

  test("connection guard rejects when disconnected", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: { guardConnection: true },
    });
    actor.mount();

    // Simulate disconnection
    frameworkMock.push({
      connection: null as never,
      connStatus: "disconnected",
    });

    const result = await actor.increment(5);
    expect(result).toBeUndefined();
    expect(actor.lastActionError?.message).toContain("disconnected");
  });

  test("connection guard waits for connecting then dispatches", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: { guardConnection: true },
    });
    actor.mount();

    frameworkMock.push({ connStatus: "connecting" });
    const pending = actor.increment(5);
    expect(actor.pendingActions).toBe(1);
    expect(actor.isMutating).toBe(true);
    frameworkMock.push({ connStatus: "connected" });
    await vi.advanceTimersByTimeAsync(0);
    const result = await pending;
    expect(result).toBe(6);
    expect(actor.lastActionError).toBeNull();
    expect(actor.pendingActions).toBe(0);
  });

  test("connection guard times out if the handshake never lands", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: { guardConnection: true, timeout: 1_000 },
    });
    actor.mount();

    frameworkMock.push({ connStatus: "connecting" });
    const pending = actor.increment(5);
    await vi.advanceTimersByTimeAsync(1_000);
    const result = await pending;
    expect(result).toBeUndefined();
    expect(actor.lastActionError?.message).toContain("not yet connected");
    expect((actor.lastActionError as { code?: string } | null)?.code).toBe(
      "ACTOR_NOT_YET_CONNECTED",
    );
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
  });

  test("cleans up tracking when onActionSettled throws", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {
        onActionSettled: () => {
          throw new Error("settled callback failed");
        },
      },
    });
    actor.mount();

    await expect(actor.increment(5)).rejects.toThrow("settled callback failed");
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
  });

  test("cleans up tracking when throwOnError predicate throws", async () => {
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {
        throwOnError: () => {
          throw new Error("predicate failed");
        },
      },
    });
    actor.mount();

    await expect(actor.failAction()).rejects.toThrow("predicate failed");
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
  });

  test("client-level actionDefaults cascade to actor-level", async () => {
    const clientLog: string[] = [];
    const rivet = createRivetKitWithClient({} as never, {
      actionDefaults: {
        onActionStart: (name: string) => clientLog.push(`client:${name}`),
        timeout: 60_000,
      },
    });

    const actorLog: string[] = [];
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {
        // Override onActionStart (actor-level wins)
        onActionStart: (name: string) => actorLog.push(`actor:${name}`),
      },
    });
    actor.mount();

    await actor.increment(5);

    // Actor-level overrode onActionStart
    expect(clientLog).toEqual([]);
    expect(actorLog).toEqual(["actor:increment"]);
  });

  test("concurrent actions track pendingActions correctly", async () => {
    const rivet = createRivetKitWithClient({} as never);

    // Replace increment with a delayed mock
    const conn = frameworkMock.currentState().connection;
    let resolveFirst: ((v: number) => void) | undefined;
    let resolveSecond: ((v: number) => void) | undefined;
    let callCount = 0;

    conn.increment = vi.fn(
      () =>
        new Promise<number>((resolve) => {
          callCount++;
          if (callCount === 1) resolveFirst = resolve;
          else resolveSecond = resolve;
        }),
    );

    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {},
    });
    actor.mount();

    const p1 = actor.increment(1);
    expect(actor.pendingActions).toBe(1);

    const p2 = actor.increment(2);
    expect(actor.pendingActions).toBe(2);
    expect(actor.isMutating).toBe(true);

    resolveFirst!(2);
    await p1;
    expect(actor.pendingActions).toBe(1);
    expect(actor.isMutating).toBe(true);

    resolveSecond!(3);
    await p2;
    expect(actor.pendingActions).toBe(0);
    expect(actor.isMutating).toBe(false);
  });

  test("older failure cannot overwrite a newer successful invocation", async () => {
    const conn = frameworkMock.currentState().connection;
    let rejectOlder: ((error: Error) => void) | undefined;
    let resolveNewer: ((value: number) => void) | undefined;
    conn.failAction = vi.fn(
      () =>
        new Promise<never>((_resolve, reject) => {
          rejectOlder = reject;
        }),
    );
    conn.increment = vi.fn(
      () =>
        new Promise<number>((resolve) => {
          resolveNewer = resolve;
        }),
    );

    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {},
    });
    actor.mount();

    const older = actor.failAction();
    const newer = actor.increment(1);
    resolveNewer!(2);
    await expect(newer).resolves.toBe(2);
    expect(actor.lastActionError).toBeNull();

    rejectOlder!(new Error("older failure"));
    await expect(older).resolves.toBeUndefined();
    expect(actor.lastActionError).toBeNull();
    expect(actor.pendingActions).toBe(0);
  });

  test("older success cannot clear a newer failed invocation", async () => {
    const conn = frameworkMock.currentState().connection;
    let resolveOlder: ((value: number) => void) | undefined;
    let rejectNewer: ((error: Error) => void) | undefined;
    conn.increment = vi.fn(
      () =>
        new Promise<number>((resolve) => {
          resolveOlder = resolve;
        }),
    );
    conn.failAction = vi.fn(
      () =>
        new Promise<never>((_resolve, reject) => {
          rejectNewer = reject;
        }),
    );

    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {},
    });
    actor.mount();

    const older = actor.increment(1);
    const newer = actor.failAction();
    rejectNewer!(new Error("newer failure"));
    await expect(newer).resolves.toBeUndefined();
    expect(actor.lastActionError?.message).toBe("newer failure");

    resolveOlder!(2);
    await expect(older).resolves.toBe(2);
    expect(actor.lastActionError?.message).toBe("newer failure");
    expect(actor.pendingActions).toBe(0);
  });

  test("late action completion cannot repopulate disposed state", async () => {
    let resolveAction: ((value: number) => void) | undefined;
    frameworkMock.currentState().connection.increment = vi.fn(
      () =>
        new Promise<number>((resolve) => {
          resolveAction = resolve;
        }),
    );
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {},
    });
    actor.mount();

    const pending = actor.increment(1);
    expect(actor.pendingActions).toBe(1);
    actor.dispose();
    expect(actor.pendingActions).toBe(0);
    expect(actor.lastAction).toBeNull();

    resolveAction!(2);
    await expect(pending).resolves.toBeUndefined();
    expect(actor.pendingActions).toBe(0);
    expect(actor.lastAction).toBeNull();
    expect(actor.lastActionError).toBeNull();
  });

  test("dispose settles an initial action through throwOnError: false", async () => {
    frameworkMock.push({ connStatus: "connecting" });
    const onError = vi.fn();
    const onSettled = vi.fn();
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.createReactiveActor({
      name: "chat" as never,
      key: ["room-1"],
      actionDefaults: {
        onActionError: onError,
        onActionSettled: onSettled,
        throwOnError: false,
      },
    });
    actor.mount();

    const pending = actor.increment(1);
    expect(actor.pendingActions).toBe(1);
    actor.dispose();

    await expect(pending).resolves.toBeUndefined();
    expect(onError).toHaveBeenCalledWith(
      expect.objectContaining({ code: "ACTOR_IDENTITY_CHANGED" }),
      "increment",
    );
    expect(onSettled).toHaveBeenCalledWith("increment");
    expect(actor.pendingActions).toBe(0);
    expect(actor.lastActionError).toBeNull();
  });

  test("re-key settles an initial useActor action through throwOnError: false", async () => {
    frameworkMock.push({ connStatus: "connecting" });
    let roomId = "room-1";
    const onError = vi.fn();
    const rivet = createRivetKitWithClient({} as never);
    const actor = rivet.useActor(() => ({
      name: "chat" as never,
      key: [roomId],
      actionDefaults: { onActionError: onError, throwOnError: false },
    }));

    const pending = actor.increment(1);
    expect(actor.pendingActions).toBe(1);
    roomId = "room-2";
    rerunEffects();

    await expect(pending).resolves.toBeUndefined();
    expect(onError).toHaveBeenCalledWith(
      expect.objectContaining({ code: "ACTOR_IDENTITY_CHANGED" }),
      "increment",
    );
    expect(actor.pendingActions).toBe(0);
    expect(actor.lastActionError).toBeNull();
    expect(frameworkMock.getOrCreateActor).toHaveBeenLastCalledWith(
      expect.objectContaining({ key: ["room-2"] }),
    );
  });

  test("same-hash reactive option refresh preserves an initial action waiter", async () => {
    frameworkMock.push({ connStatus: "connecting" });
    let token = "token-1";
    const rivet = createRivetKitWithClient({} as never, {
      hashFunction: (opts) =>
        JSON.stringify({ name: opts.name, key: opts.key }),
    });
    const actor = rivet.useActor(() => ({
      name: "chat" as never,
      key: ["room-1"],
      params: { token },
      actionDefaults: {},
    }));

    const pending = actor.increment(1);
    expect(actor.pendingActions).toBe(1);
    token = "token-2";
    rerunEffects();

    expect(actor.pendingActions).toBe(1);
    expect(actor.lastAction).toBe("increment");
    frameworkMock.push({ connStatus: "connected" });
    await vi.advanceTimersByTimeAsync(0);

    await expect(pending).resolves.toBe(2);
    expect(actor.pendingActions).toBe(0);
    expect(actor.lastActionError).toBeNull();
  });
});
