import "./runes-shim.js";
import { describe, expect, test, vi } from "vitest";

vi.mock("../internal/framework-base.js", () => ({
  createRivetKit: vi.fn(() => ({
    getOrCreateActor: vi.fn(),
  })),
}));

vi.mock("esm-env", () => ({
  BROWSER: true,
  DEV: false,
}));

import { createRivetKitWithClient } from "../rivetkit.svelte.js";

function createClient(
  resolveImpl: () => Promise<string> = () => Promise.resolve("actor-id"),
) {
  const resolve = vi.fn(resolveImpl);
  const handle = { resolve };
  const get = vi.fn(() => handle);
  const getOrCreate = vi.fn(() => handle);
  const client = {
    document: { get, getOrCreate },
  };

  return { client, get, getOrCreate, resolve };
}

describe("warmUp", () => {
  test("resolves actor with getOrCreate without opening a connection", () => {
    const { client, get, getOrCreate, resolve } = createClient();
    const rivet = createRivetKitWithClient(client as never);

    rivet.warmUp({ name: "document" as never, key: "doc-1" });

    expect(getOrCreate).toHaveBeenCalledWith(["doc-1"], {});
    expect(get).not.toHaveBeenCalled();
    expect(resolve).toHaveBeenCalledTimes(1);
  });

  test("passes null createWithInput to Rivet", () => {
    const { client, getOrCreate } = createClient();
    const rivet = createRivetKitWithClient(client as never);

    rivet.warmUp({
      name: "document" as never,
      key: ["doc-1"],
      createWithInput: null,
    });

    expect(getOrCreate).toHaveBeenCalledWith(["doc-1"], {
      createWithInput: null,
    });
  });

  test("passes createInRegion to getOrCreate", () => {
    const { client, getOrCreate } = createClient();
    const rivet = createRivetKitWithClient(client as never);

    rivet.warmUp({
      name: "document" as never,
      key: ["doc-1"],
      createInRegion: "atl",
    });

    expect(getOrCreate).toHaveBeenCalledWith(["doc-1"], {
      createInRegion: "atl",
    });
  });

  test("uses get when noCreate is requested", () => {
    const { client, get, getOrCreate, resolve } = createClient();
    const rivet = createRivetKitWithClient(client as never);

    rivet.warmUp({
      name: "document" as never,
      key: ["doc-1"],
      noCreate: true,
    });

    expect(get).toHaveBeenCalledWith(["doc-1"]);
    expect(getOrCreate).not.toHaveBeenCalled();
    expect(resolve).toHaveBeenCalledTimes(1);
  });

  test("deduplicates concurrent warm-ups by actor identity", () => {
    const { client, resolve } = createClient(
      () => new Promise<string>(() => undefined),
    );
    const rivet = createRivetKitWithClient(client as never);

    rivet.warmUp({ name: "document" as never, key: ["doc-1"] });
    rivet.warmUp({ name: "document" as never, key: ["doc-1"] });

    expect(resolve).toHaveBeenCalledTimes(1);
  });

  test("allows a later warm-up after the previous resolve completes", async () => {
    const { client, resolve } = createClient();
    const rivet = createRivetKitWithClient(client as never);

    rivet.warmUp({ name: "document" as never, key: ["doc-1"] });
    // Let the resolved promise's completion handler clear the in-flight key.
    await Promise.resolve();
    rivet.warmUp({ name: "document" as never, key: ["doc-1"] });

    expect(resolve).toHaveBeenCalledTimes(2);
  });

  test("allows retry after resolve failure", async () => {
    let rejectResolve: ((error: Error) => void) | undefined;
    const { client, resolve } = createClient(
      () =>
        new Promise<string>((_resolve, reject) => {
          rejectResolve = reject;
        }),
    );
    const rivet = createRivetKitWithClient(client as never);

    rivet.warmUp({ name: "document" as never, key: ["doc-1"] });
    expect(resolve).toHaveBeenCalledTimes(1);

    rejectResolve!(new Error("resolve failed"));
    await Promise.resolve();
    rivet.warmUp({ name: "document" as never, key: ["doc-1"] });
    expect(resolve).toHaveBeenCalledTimes(2);
  });

  test("supports cyclic warm-up input without leaking hashing failures", () => {
    const { client, resolve } = createClient();
    const rivet = createRivetKitWithClient(client as never);
    const circular: { self?: unknown } = {};
    circular.self = circular;

    expect(() =>
      rivet.warmUp({
        name: "document" as never,
        key: ["doc-1"],
        createWithInput: circular,
      }),
    ).not.toThrow();
    expect(resolve).toHaveBeenCalledTimes(1);
  });

  test("supports BigInt warm-up input", () => {
    const { client, resolve } = createClient();
    const rivet = createRivetKitWithClient(client as never);

    expect(() =>
      rivet.warmUp({
        name: "document" as never,
        key: ["doc-1"],
        createWithInput: { cursor: 1n },
      }),
    ).not.toThrow();
    expect(resolve).toHaveBeenCalledTimes(1);
  });

  test("contains synchronous resolve failures and permits retry", () => {
    let shouldThrow = true;
    const { client, resolve } = createClient(() => {
      if (shouldThrow) throw new Error("sync resolve failed");
      return Promise.resolve("actor-id");
    });
    const rivet = createRivetKitWithClient(client as never);

    expect(() =>
      rivet.warmUp({ name: "document" as never, key: ["doc-1"] }),
    ).not.toThrow();
    shouldThrow = false;
    rivet.warmUp({ name: "document" as never, key: ["doc-1"] });

    expect(resolve).toHaveBeenCalledTimes(2);
  });

  test("deprecated preloadActor alias still resolves the actor", () => {
    const { client, getOrCreate, resolve } = createClient();
    const rivet = createRivetKitWithClient(client as never);

    rivet.preloadActor({ name: "document" as never, key: "doc-1" });

    expect(getOrCreate).toHaveBeenCalledWith(["doc-1"], {});
    expect(resolve).toHaveBeenCalledTimes(1);
  });
});
