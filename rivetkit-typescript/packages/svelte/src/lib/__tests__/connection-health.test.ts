import "./runes-shim.js";
import { describe, expect, test } from "vitest";
import { createConnectionHealth } from "../connection-health.svelte.js";

describe("createConnectionHealth", () => {
  test("reads each source getter once per snapshot", () => {
    let statusReads = 0;
    let errorReads = 0;
    const source = {
      get connStatus() {
        statusReads += 1;
        return "connected";
      },
      get error() {
        errorReads += 1;
        return null;
      },
    };

    const health = createConnectionHealth(() => ({ chat: source }));

    expect(health.status).toBe("connected");
    expect(health.actors.chat.status).toBe("connected");
    expect(statusReads).toBe(1);
    expect(errorReads).toBe(1);
  });

  test("distinguishes connected, degraded, connecting, and offline", () => {
    const statusFor = (chat: string, inbox: string) =>
      createConnectionHealth(() => ({
        chat: { connStatus: chat, error: null },
        inbox: { connStatus: inbox, error: null },
      })).status;

    expect(statusFor("connected", "connected")).toBe("connected");
    expect(statusFor("connected", "disconnected")).toBe("degraded");
    expect(statusFor("connecting", "disconnected")).toBe("connecting");
    expect(statusFor("disconnected", "disconnected")).toBe("offline");
  });
});
