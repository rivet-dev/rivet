import { afterEach, expect, test, vi } from "vitest";
import { ClientConfigSchema } from "@/client/config";
import { RemoteEngineControlClient } from "@/engine-client/mod";

afterEach(() => vi.unstubAllGlobals());

test("a rejected WebSocket handshake renews the token before reconnecting", async () => {
	const getToken = vi.fn(
		async ({ forceRefresh }: { forceRefresh: boolean }) =>
			forceRefresh ? "replacement" : "first",
	);
	const sockets: FakeWebSocket[] = [];
	vi.stubGlobal(
		"WebSocket",
		class extends FakeWebSocket {
			constructor(url: string | URL, protocols?: string | string[]) {
				super(url, protocols);
				sockets.push(this);
			}
		},
	);
	const driver = new RemoteEngineControlClient(
		ClientConfigSchema.parse({
			endpoint: "https://api.rivet.dev",
			disableMetadataLookup: true,
			getToken,
		}),
	);
	const first = await driver.openWebSocket(
		"/connect",
		{ directId: "one" },
		"bare",
		undefined,
	);
	expect(sockets[0]?.url).toContain("@first/connect");
	sockets[0]?.emitClose("auth.token_expired");
	expect(await driver.refreshAuthToken(first)).toBe(true);
	await driver.openWebSocket(
		"/connect",
		{ directId: "one" },
		"bare",
		undefined,
	);
	expect(sockets[1]?.url).toContain("@replacement/connect");
	expect(getToken).toHaveBeenCalledTimes(2);
	const replacement = sockets[1];
	if (!replacement) throw new Error("missing replacement socket");
	replacement.emitClose("auth.insufficient_permissions");
	expect(await driver.refreshAuthToken(replacement)).toBe(false);
	expect(getToken).toHaveBeenCalledTimes(2);
});

class FakeWebSocket {
	readonly url: string;
	readonly protocols: string | string[] | undefined;
	readonly readyState = 0;
	binaryType = "blob";
	#listeners = new Map<
		string,
		Array<(event: { code?: number; reason?: string }) => void>
	>();

	constructor(url: string | URL, protocols?: string | string[]) {
		this.url = String(url);
		this.protocols = protocols;
	}

	addEventListener(
		type: string,
		listener: (event: { code?: number; reason?: string }) => void,
	): void {
		const listeners = this.#listeners.get(type) ?? [];
		listeners.push(listener);
		this.#listeners.set(type, listeners);
	}
	removeEventListener(): void {}
	send(): void {}
	close(): void {}

	emitClose(reason: string): void {
		for (const listener of this.#listeners.get("close") ?? [])
			listener({ code: 1008, reason });
	}
}
