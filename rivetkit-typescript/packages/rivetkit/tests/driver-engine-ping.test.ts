/**
 * Simple smoke test that verifies the native envoy client can connect,
 * create an actor, handle an HTTP request, and handle a WebSocket echo.
 *
 * Requires a running engine at RIVET_ENDPOINT (default http://localhost:6420)
 * and a test-envoy with pool name "test-envoy" in the "default" namespace.
 */
import { describe, it, expect } from "vitest";

const RIVET_ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://localhost:6420";
const RIVET_TOKEN = process.env.RIVET_TOKEN ?? "dev";
const RIVET_NAMESPACE = process.env.RIVET_NAMESPACE ?? "default";
const RUNNER_NAME = "test-envoy";

async function createActor(): Promise<{ actor_id: string }> {
	const response = await fetch(
		`${RIVET_ENDPOINT}/actors?namespace=${RIVET_NAMESPACE}`,
		{
			method: "POST",
			headers: {
				Authorization: `Bearer ${RIVET_TOKEN}`,
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				name: "thingy",
				key: crypto.randomUUID(),
				input: btoa("hello"),
				runner_name_selector: RUNNER_NAME,
				crash_policy: "sleep",
			}),
		},
	);

	if (!response.ok) {
		const text = await response.text();
		throw new Error(`Create actor failed: ${response.status} ${text}`);
	}

	const body = await response.json();
	return { actor_id: body.actor.actor_id };
}

async function destroyActor(actorId: string): Promise<void> {
	await fetch(
		`${RIVET_ENDPOINT}/actors/${actorId}?namespace=${RIVET_NAMESPACE}`,
		{
			method: "DELETE",
			headers: { Authorization: `Bearer ${RIVET_TOKEN}` },
		},
	);
}

describe("engine driver smoke test", () => {
	it("HTTP ping returns JSON response", async () => {
		const { actor_id } = await createActor();
		try {
			const response = await fetch(`${RIVET_ENDPOINT}/ping`, {
				method: "GET",
				headers: {
					"X-Rivet-Token": RIVET_TOKEN,
					"X-Rivet-Target": "actor",
					"X-Rivet-Actor": actor_id,
				},
			});

			expect(response.ok).toBe(true);
			const body = await response.json();
			expect(body.actorId).toBe(actor_id);
			expect(body.status).toBe("ok");
		} finally {
			await destroyActor(actor_id);
		}
	}, 30_000);

	it("WebSocket echo works", async () => {
		const { actor_id } = await createActor();
		try {
			const wsEndpoint = RIVET_ENDPOINT.replace(
				"http://",
				"ws://",
			).replace("https://", "wss://");
			const ws = new WebSocket(`${wsEndpoint}/ws`, [
				"rivet",
				"rivet_target.actor",
				`rivet_actor.${actor_id}`,
				`rivet_token.${RIVET_TOKEN}`,
			]);

			const result = await new Promise<string>((resolve, reject) => {
				const timeout = setTimeout(
					() => reject(new Error("WebSocket timeout")),
					10_000,
				);

				ws.addEventListener("open", () => {
					ws.send("ping");
				});

				ws.addEventListener("message", (event) => {
					clearTimeout(timeout);
					ws.close();
					resolve(event.data as string);
				});

				ws.addEventListener("error", (e) => {
					clearTimeout(timeout);
					reject(
						new Error(
							`WebSocket error: ${(e as any)?.message ?? "unknown"}`,
						),
					);
				});
			});

			expect(result).toBe("Echo: ping");
		} finally {
			await destroyActor(actor_id);
		}
	}, 30_000);
});
