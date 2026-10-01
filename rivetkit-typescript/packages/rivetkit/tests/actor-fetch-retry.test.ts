import { describe, expect, test } from "vitest";
import { ActorHandleRaw } from "@/client/actor-handle";
import { ActorError } from "@/client/errors";
import type {
	EngineControlClient,
	GatewayTarget,
} from "@/engine-client/driver";

const ENVOY_ADMISSION_ERRORS = [
	["actor_not_found", "Actor not found"],
	["actor_generation_mismatch", "Actor generation does not match"],
] as const;

function envoyAdmissionErrorResponse(code: string, message: string) {
	return Response.json(
		{
			group: "envoy",
			code,
			message,
			actor: { actorId: "actor-id", generation: 7 },
		},
		{
			status: 503,
			headers: { "x-rivet-error": `envoy.${code}` },
		},
	);
}

function dynamicHandle(driver: EngineControlClient) {
	return new ActorHandleRaw({}, driver, undefined, undefined, "json", {
		getOrCreateForKey: { name: "example", key: ["key"] },
	});
}

describe("ActorHandleRaw.fetch", () => {
	test("clones a Request body so a retry can re-send it", async () => {
		const bodies: string[] = [];
		let attempts = 0;
		const driver = {
			async sendRequest(_target: GatewayTarget, request: Request) {
				attempts++;
				bodies.push(await request.text());
				// Fail the first attempt with a retryable lifecycle error so the
				// loop re-issues the request, then succeed.
				if (attempts === 1) {
					throw new ActorError(
						"actor",
						"starting",
						"actor is starting",
					);
				}
				return Response.json({ ok: true });
			},
		} as EngineControlClient;
		const handle = new ActorHandleRaw(
			{},
			driver,
			undefined,
			undefined,
			"json",
			{ getOrCreateForKey: { name: "example", key: ["key"] } },
		);
		const request = new Request("http://example.test/submit", {
			method: "POST",
			body: "persistent request body",
		});

		const response = await handle.fetch(request);

		expect(response.ok).toBe(true);
		expect(attempts).toBe(2);
		// Each attempt clones the caller's Request, so the retry re-sends the
		// full body rather than an empty/disturbed stream.
		expect(bodies).toEqual([
			"persistent request body",
			"persistent request body",
		]);
		// The caller's Request is never consumed directly, so it stays intact
		// and can be inspected or re-issued after the call.
		expect(request.bodyUsed).toBe(false);
	});

	test("retries and re-sends a string init body", async () => {
		const bodies: string[] = [];
		let attempts = 0;
		const driver = {
			async sendRequest(_target: GatewayTarget, request: Request) {
				attempts++;
				bodies.push(await request.text());
				if (attempts === 1) {
					throw new ActorError(
						"actor",
						"starting",
						"actor is starting",
					);
				}
				return Response.json({ ok: true });
			},
		} as EngineControlClient;
		const handle = dynamicHandle(driver);

		const response = await handle.fetch("http://example.test/submit", {
			method: "POST",
			body: "body from init",
		});

		expect(response.ok).toBe(true);
		expect(attempts).toBe(2);
		// A string init body is re-sendable, so the retry delivers it again.
		expect(bodies).toEqual(["body from init", "body from init"]);
	});

	test("buffers a streaming init body so a retry can re-send it", async () => {
		const bodies: string[] = [];
		let attempts = 0;
		const driver = {
			async sendRequest(_target: GatewayTarget, request: Request) {
				attempts++;
				bodies.push(await request.text());
				if (attempts === 1) {
					throw new ActorError(
						"actor",
						"starting",
						"actor is starting",
					);
				}
				return Response.json({ ok: true });
			},
		} as EngineControlClient;
		const handle = dynamicHandle(driver);

		const stream = new ReadableStream({
			start(controller) {
				controller.enqueue(new TextEncoder().encode("streamed body"));
				controller.close();
			},
		});

		const response = await handle.fetch("http://example.test/submit", {
			method: "POST",
			body: stream,
		});

		expect(response.ok).toBe(true);
		expect(attempts).toBe(2);
		// The one-shot stream is buffered up front, so the retry re-sends the
		// full payload instead of a disturbed stream.
		expect(bodies).toEqual(["streamed body", "streamed body"]);
	});

	test("rejects an already-aborted streaming init body before dispatch", async () => {
		let attempts = 0;
		const driver = {
			async sendRequest() {
				attempts++;
				return Response.json({ ok: true });
			},
		} as EngineControlClient;
		const handle = dynamicHandle(driver);
		const controller = new AbortController();
		controller.abort();
		const stream = new ReadableStream({
			start(streamController) {
				streamController.enqueue(
					new TextEncoder().encode("streamed body"),
				);
			},
		});

		await expect(
			handle.fetch("http://example.test/submit", {
				method: "POST",
				body: stream,
				signal: controller.signal,
			}),
		).rejects.toBe(controller.signal.reason);
		expect(attempts).toBe(0);
		expect(stream.locked).toBe(false);
	});

	test("cancels stream buffering when the request signal aborts", async () => {
		let attempts = 0;
		let cancelReason: unknown;
		const driver = {
			async sendRequest() {
				attempts++;
				return Response.json({ ok: true });
			},
		} as EngineControlClient;
		const handle = new ActorHandleRaw(
			{},
			driver,
			undefined,
			undefined,
			"json",
			{ getForKey: { name: "example", key: ["key"] } },
		);
		const controller = new AbortController();
		const stream = new ReadableStream({
			start(streamController) {
				streamController.enqueue(
					new TextEncoder().encode("streamed body"),
				);
			},
			cancel(reason) {
				cancelReason = reason;
			},
		});
		const request = handle.fetch("http://example.test/submit", {
			method: "POST",
			body: stream,
			signal: controller.signal,
		});
		const reason = new Error("cancel upload");

		controller.abort(reason);

		await expect(request).rejects.toBe(reason);
		expect(cancelReason).toBe(reason);
		expect(attempts).toBe(0);
		expect(stream.locked).toBe(false);
	});

	test("sends an init body override without cloning a consumed Request", async () => {
		const bodies: string[] = [];
		const driver = {
			async sendRequest(_target: GatewayTarget, request: Request) {
				bodies.push(await request.text());
				return Response.json({ ok: true });
			},
		} as EngineControlClient;
		const handle = dynamicHandle(driver);

		const request = new Request("http://example.test/submit", {
			method: "POST",
			body: "original body",
		});
		// Consume the caller's Request, then replace its body through init.
		await request.text();
		expect(request.bodyUsed).toBe(true);

		const response = await handle.fetch(request, { body: "replacement" });

		expect(response.ok).toBe(true);
		// The replacement body is sent; the consumed input is never cloned.
		expect(bodies).toEqual(["replacement"]);
	});

	test("retains lifecycle retries for bodyless requests", async () => {
		let attempts = 0;
		const driver = {
			async sendRequest() {
				attempts++;
				if (attempts === 1) {
					throw new ActorError(
						"actor",
						"starting",
						"actor is starting",
					);
				}
				return Response.json({ ok: true });
			},
		} as EngineControlClient;
		const handle = dynamicHandle(driver);

		const response = await handle.fetch("http://example.test/status");

		expect(response.ok).toBe(true);
		expect(attempts).toBe(2);
	});

	test.each(
		ENVOY_ADMISSION_ERRORS,
	)("surfaces envoy.%s as an ActorError without retrying an action", async (code, message) => {
		let attempts = 0;
		const driver = {
			async sendRequest() {
				attempts++;
				return envoyAdmissionErrorResponse(code, message);
			},
		} as EngineControlClient;
		const handle = dynamicHandle(driver);

		let error: unknown;
		try {
			await handle.action({ name: "test", args: [] });
		} catch (cause) {
			error = cause;
		}

		expect(error).toBeInstanceOf(ActorError);
		expect(error).toMatchObject({
			group: "envoy",
			code,
			message,
			actor: { actorId: "actor-id", generation: 7 },
		});
		expect(attempts).toBe(1);
	});

	test.each(
		ENVOY_ADMISSION_ERRORS,
	)("keeps an envoy.%s raw fetch response readable without retrying", async (code, message) => {
		let attempts = 0;
		const driver = {
			async sendRequest() {
				attempts++;
				return envoyAdmissionErrorResponse(code, message);
			},
		} as EngineControlClient;
		const handle = dynamicHandle(driver);

		const response = await handle.fetch("http://actor/request");

		expect(response.ok).toBe(false);
		expect(response.status).toBe(503);
		expect(response.headers.get("content-type")).toContain(
			"application/json",
		);
		expect(response.headers.get("x-rivet-error")).toBe(`envoy.${code}`);
		expect(await response.json()).toEqual({
			group: "envoy",
			code,
			message,
			actor: { actorId: "actor-id", generation: 7 },
		});
		expect(attempts).toBe(1);
	});
});
