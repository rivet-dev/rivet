/**
 * Runtime contract for the package-local framework bridge.
 *
 * Expiring actor credentials must reach the Rivet client as `getParams` for
 * both existing-only and create-if-missing access. This test exercises the
 * actual bridge shipped inside `@rivetkit/svelte`; it deliberately does not
 * inspect a workspace package-manager patch.
 */

import { describe, expect, test, vi } from "vitest";
import { createRivetKit } from "../internal/framework-base.js";

function connection() {
  return {
    connStatus: "connecting",
    onStatusChange: () => () => {},
    onError: () => () => {},
    dispose: () => {},
  };
}

function clientHarness() {
  const connect = vi.fn(() => connection());
  const handle = { connect };
  const get = vi.fn(() => handle);
  const getOrCreate = vi.fn(() => handle);
  return {
    client: { get, getOrCreate },
    connect,
    get,
    getOrCreate,
  };
}

describe("package-local framework getParams forwarding", () => {
  test("forwards a fresh-params resolver through getOrCreate", () => {
    const harness = clientHarness();
    const getParams = vi.fn(async () => ({ token: "fresh" }));
    const framework = createRivetKit(harness.client as never);
    const actor = framework.getOrCreateActor({
      name: "document" as never,
      key: ["document", "doc-1"],
      getParams,
    });
    const unmount = actor.mount();

    expect(harness.getOrCreate).toHaveBeenCalledWith(
      "document",
      ["document", "doc-1"],
      expect.objectContaining({ getParams }),
    );
    expect(harness.connect).toHaveBeenCalledTimes(1);

    unmount();
  });

  test("forwards a fresh-params resolver through existing-only get", () => {
    const harness = clientHarness();
    const getParams = vi.fn(async () => ({ token: "fresh" }));
    const framework = createRivetKit(harness.client as never);
    const actor = framework.getOrCreateActor({
      name: "document" as never,
      key: ["document", "doc-2"],
      noCreate: true,
      getParams,
    });
    const unmount = actor.mount();

    expect(harness.get).toHaveBeenCalledWith(
      "document",
      ["document", "doc-2"],
      expect.objectContaining({ getParams }),
    );
    expect(harness.connect).toHaveBeenCalledTimes(1);

    unmount();
  });
});
