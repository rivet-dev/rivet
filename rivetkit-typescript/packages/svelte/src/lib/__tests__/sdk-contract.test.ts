import { ActorError, createClient, UserError } from "rivetkit/client";
import { afterEach, describe, expect, test, vi } from "vitest";
import {
  actorErrorCode,
  actorErrorMessage,
  getActionError,
  isActorError,
} from "../errors.js";

// Keep the SDK unmocked: these contracts must hold in standalone installs.
afterEach(() => vi.unstubAllGlobals());

describe("public RivetKit SDK contract", () => {
  test("recognizes real SDK errors and preserves structured details", () => {
    const error = new ActorError("user", "RATE_LIMITED", "Try again later", {
      public: true,
      metadata: { retryAfterMs: 100 },
    });

    expect(isActorError(error)).toBe(true);
    expect(actorErrorCode(error)).toBe("RATE_LIMITED");
    expect(actorErrorMessage(error)).toBe("Try again later");
    expect(error.metadata).toEqual({ retryAfterMs: 100 });
    expect(getActionError({ lastActionError: error })).toEqual({
      message: "Try again later",
      code: "RATE_LIMITED",
      isActorError: true,
    });
    expect(isActorError(new UserError("Invalid input", { code: "INVALID" }))).toBe(true);
  });

  test.each([undefined, "ActorError", "RivetError"])(
    "recognizes a serialized SDK error with discriminator %s",
    (__type) => {
      const error = {
        ...(__type === undefined ? {} : { __type }),
        group: "user",
        code: "INVALID",
        message: "Invalid input",
      };
      expect(ActorError.isActorError(error)).toBe(true);
      expect(isActorError(error)).toBe(true);
      expect(actorErrorCode(error)).toBe("INVALID");
    },
  );

  test.each([null, undefined, new Error("Network failed"), {}, {
    __type: "RivetError", group: "user", code: 42, message: "Invalid",
  }])("rejects non-actor errors safely: %s", (error) => {
    expect(isActorError(error)).toBe(false);
    expect(actorErrorCode(error)).toBeUndefined();
  });

  test("accepts raw action options through the public HTTP transport", async () => {
    const requests: Request[] = [];
    vi.stubGlobal("fetch", async (input: RequestInfo | URL, init?: RequestInit) => {
      requests.push(new Request(input, init));
      return Response.json({ output: 7 });
    });
    const client = createClient({
      endpoint: "http://localhost:6420",
      encoding: "json",
      disableMetadataLookup: true,
    });
    const handle = client.getForId("counter", "counter-1");
    const controller = new AbortController();

    await expect(handle.action({
      name: "increment",
      args: [6],
      signal: controller.signal,
    })).resolves.toBe(7);
    expect(requests).toHaveLength(1);
    expect(requests[0].method).toBe("POST");
    expect(requests[0].url).toContain("/action/increment");
    expect(await requests[0].json()).toEqual({ args: [6] });
    controller.abort();
    expect(requests[0].signal.aborted).toBe(true);
  });

  test("extracts an error decoded by the real SDK from an action response", async () => {
    vi.stubGlobal("fetch", async () => Response.json({
      group: "user",
      code: "INVALID_INPUT",
      message: "A positive number is required",
      metadata: { field: "amount" },
    }, { status: 400, headers: { "x-rivet-ray-id": "request-1" } }));
    const client = createClient({
      endpoint: "http://localhost:6420",
      encoding: "json",
      disableMetadataLookup: true,
    });
    const error = await client.getForId("counter", "counter-1")
      .action({ name: "increment", args: [-1] })
      .catch((cause: unknown) => cause);

    expect(error).toBeInstanceOf(ActorError);
    expect(getActionError({ lastActionError: error })).toEqual({
      message: "A positive number is required",
      code: "INVALID_INPUT",
      isActorError: true,
    });
    expect(error).toMatchObject({
      metadata: { field: "amount" },
      rayId: "request-1",
    });
  });
});
