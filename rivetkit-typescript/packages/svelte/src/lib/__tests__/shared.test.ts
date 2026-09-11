import "./runes-shim.js";
import { describe, expect, test, vi } from "vitest";
import type { AnyActorRegistry } from "../index.js";
import { ActorError } from "rivetkit/client";
import {
  createReactiveConnection,
  createSharedRivetKit,
  withActorParams,
  getActionError,
} from "../index.js";
import { createMockConnection } from "./helpers.js";

describe("shared helpers", () => {
  test("createSharedRivetKit reuses a single wrapper", () => {
    const client = { id: "client" } as never;
    let clientCalls = 0;

    const getRivet = createSharedRivetKit<AnyActorRegistry>(() => {
      clientCalls += 1;
      return client;
    });

    const a = getRivet();
    const b = getRivet();

    expect(a).toBe(b);
    expect(clientCalls).toBe(1);
  });

  test("withActorParams merges static and getter params", () => {
    let token = "first";

    const getOpts = withActorParams<AnyActorRegistry, never>(
      {
        name: "chat" as never,
        key: ["room-1"],
        params: { organizationId: "org-1" },
      },
      () => ({ token }),
    );

    expect(getOpts()).toEqual({
      name: "chat",
      key: ["room-1"],
      params: { organizationId: "org-1", token: "first" },
    });

    token = "second";

    expect(getOpts().params).toEqual({
      organizationId: "org-1",
      token: "second",
    });
  });

  test("withActorParams omits params when both inputs are undefined", () => {
    const getOpts = withActorParams<AnyActorRegistry, never>(
      {
        name: "chat" as never,
        key: ["room-1"],
      },
      () => undefined,
    );

    expect(getOpts()).toEqual({
      name: "chat",
      key: ["room-1"],
    });
  });

  test("createReactiveConnection reflects status, errors, and events", async () => {
    const mock = createMockConnection();
    const reactive = createReactiveConnection({
      connect: () => mock.connection,
    });

    expect(reactive.connStatus).toBe("idle");
    expect(reactive.isConnected).toBe(false);

    reactive.connect();
    mock.setStatus("connected");

    expect(reactive.connStatus).toBe("connected");
    expect(reactive.isConnected).toBe(true);

    let payload: string | null = null;
    const unsubscribe = reactive.onEvent("message", (value) => {
      payload = value as string;
    });

    mock.emit("message", "hello");
    expect(payload).toBe("hello");

    mock.emitError("boom");
    expect(reactive.error?.message).toBe("boom");

    unsubscribe();
    await reactive.dispose();

    expect(reactive.connStatus).toBe("disconnected");
    expect(reactive.connection).toBe(null);
  });

  test("whenConnected resolves true when status becomes connected", async () => {
    const mock = createMockConnection();
    const reactive = createReactiveConnection({
      connect: () => mock.connection,
    });

    reactive.connect();

    const promise = reactive.whenConnected(5_000);
    mock.setStatus("connected");

    const result = await promise;
    expect(result).toBe(true);
  });

  test("connect resolves existing waiters when the source is already connected", async () => {
    const mock = createMockConnection();
    mock.setStatus("connected");
    const reactive = createReactiveConnection({
      connect: () => mock.connection,
    });

    const promise = reactive.whenConnected(5_000);
    reactive.connect();

    await expect(promise).resolves.toBe(true);
  });

  test("whenConnected resolves false on timeout", async () => {
    vi.useFakeTimers();

    const mock = createMockConnection();
    const reactive = createReactiveConnection({
      connect: () => mock.connection,
    });

    reactive.connect();

    const promise = reactive.whenConnected(100);
    vi.advanceTimersByTime(150);

    const result = await promise;
    expect(result).toBe(false);

    vi.useRealTimers();
  });

  test("disconnect cancels pending whenConnected with false", async () => {
    const mock = createMockConnection();
    const reactive = createReactiveConnection({
      connect: () => mock.connection,
    });

    reactive.connect();

    const promise = reactive.whenConnected(30_000);
    await reactive.disconnect();

    const result = await promise;
    expect(result).toBe(false);
  });

  test("disconnect cancels a waiter registered before connect", async () => {
    const reactive = createReactiveConnection({
      connect: () => createMockConnection().connection,
    });

    const pending = reactive.whenConnected(30_000);
    await reactive.disconnect();

    await expect(pending).resolves.toBe(false);
  });

  test("clears connection state even when transport disposal fails", async () => {
    const mock = createMockConnection();
    mock.connection.dispose = vi.fn(async () => {
      throw new Error("dispose failed");
    });
    const reactive = createReactiveConnection({
      connect: () => mock.connection,
    });
    reactive.connect();

    await expect(reactive.disconnect()).rejects.toThrow("dispose failed");
    expect(reactive.connection).toBeNull();
    expect(reactive.connStatus).toBe("disconnected");
    expect(reactive.error).toBeNull();
  });

  test("disconnect retains event registrations for reconnect", async () => {
    const first = createMockConnection();
    const second = createMockConnection();
    const connect = vi
      .fn()
      .mockReturnValueOnce(first.connection)
      .mockReturnValueOnce(second.connection);
    const reactive = createReactiveConnection({
      connect,
    });
    const handler = vi.fn();

    reactive.onEvent("message", handler);
    reactive.connect();
    await reactive.disconnect();
    reactive.connect();
    second.emit("message", "after-reconnect");

    expect(handler).toHaveBeenCalledWith("after-reconnect");
  });

  test("dispose stays reusable and shares a slow in-flight teardown", async () => {
    const first = createMockConnection();
    const second = createMockConnection();
    let finishDispose: (() => void) | undefined;
    first.connection.dispose = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          finishDispose = resolve;
        }),
    );
    const connect = vi
      .fn()
      .mockReturnValueOnce(first.connection)
      .mockReturnValueOnce(second.connection);
    const reactive = createReactiveConnection({ connect });
    const handler = vi.fn();
    reactive.onEvent("message", handler);
    reactive.connect();

    const disposing = reactive.dispose();
    expect(reactive.dispose()).toBe(disposing);

    expect(reactive.connection).toBeNull();
    expect(reactive.connStatus).toBe("disconnected");
    expect(connect).toHaveBeenCalledTimes(1);

    finishDispose!();
    await disposing;

    reactive.connect();
    second.emit("message", "after-dispose");
    expect(handler).toHaveBeenCalledWith("after-dispose");
    expect(connect).toHaveBeenCalledTimes(2);
  });

  test("a failed disposal still permits reconnect and retained events", async () => {
    const first = createMockConnection();
    const second = createMockConnection();
    first.connection.dispose = vi.fn(async () => {
      throw new Error("dispose failed");
    });
    const connect = vi
      .fn()
      .mockReturnValueOnce(first.connection)
      .mockReturnValueOnce(second.connection);
    const reactive = createReactiveConnection({
      connect,
    });
    const handler = vi.fn();
    reactive.onEvent("message", handler);
    reactive.connect();

    await expect(reactive.dispose()).rejects.toThrow("dispose failed");

    reactive.connect();
    second.emit("message", "after-failure");
    expect(handler).toHaveBeenCalledWith("after-failure");
    expect(connect).toHaveBeenCalledTimes(2);
  });

  test("cleanup failures reject asynchronously and do not skip socket disposal", async () => {
    const mock = createMockConnection();
    const closeSocket = vi.fn(async () => {});
    mock.connection.on = vi.fn(() => () => {
      throw new Error("unsubscribe failed");
    });
    mock.connection.dispose = closeSocket;
    const reactive = createReactiveConnection({
      connect: () => mock.connection,
    });
    reactive.onEvent("message", vi.fn());
    reactive.connect();

    const disposal = reactive.dispose();

    await expect(disposal).rejects.toThrow("unsubscribe failed");
    expect(closeSocket).toHaveBeenCalledTimes(1);
    expect(reactive.connection).toBeNull();
  });
});

describe("getActionError", () => {
  test("returns null when no error", () => {
    const result = getActionError({ lastActionError: null });
    expect(result).toBe(null);
  });

  test("returns null for undefined error", () => {
    const result = getActionError({ lastActionError: undefined });
    expect(result).toBe(null);
  });

  test("extracts message from plain Error", () => {
    const result = getActionError({
      lastActionError: new Error("something broke"),
    });
    expect(result).not.toBe(null);
    expect(result!.message).toBe("something broke");
    expect(result!.code).toBeUndefined();
    expect(result!.isActorError).toBe(false);
  });

  test("extracts code and message from a real Rivet ActorError", () => {
    const err = new ActorError("client", "RATE_LIMITED", "rate limited");
    const result = getActionError({ lastActionError: err });
    expect(result).not.toBe(null);
    expect(result!.message).toBe("rate limited");
    expect(result!.code).toBe("RATE_LIMITED");
    expect(result!.isActorError).toBe(true);
  });

  test("recognizes serialized modern and legacy actor errors", () => {
    for (const __type of ["RivetError", "ActorError"] as const) {
      const result = getActionError({
        lastActionError: {
          __type,
          group: "user",
          code: "FORBIDDEN",
          message: "not allowed",
        },
      });

      expect(result).toEqual({
        message: "not allowed",
        code: "FORBIDDEN",
        isActorError: true,
      });
    }
  });
});
