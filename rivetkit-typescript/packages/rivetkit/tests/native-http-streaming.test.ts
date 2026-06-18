import { describe, expect, test } from "vitest";
import { nativeRegistryTestInternals } from "../src/registry/native";

describe("native http response streaming", () => {
	test("constructs requests with streaming bodies and abort signals", async () => {
		const chunks = [new Uint8Array([1]), new Uint8Array([2])];
		const controller = new AbortController();
		const request = nativeRegistryTestInternals.buildRequest({
			method: "POST",
			uri: "/upload",
			body: chunks.shift(),
			bodyStream: {
				async read() {
					return chunks.shift() ?? null;
				},
				async cancel() {},
			},
			signal: controller.signal,
		});

		controller.abort();

		expect(request.signal.aborted).toBe(true);
		expect(new Uint8Array(await request.arrayBuffer())).toEqual(
			new Uint8Array([1, 2]),
		);
	});

	test("preserves native request stream chunks through Request bodies", async () => {
		const chunkSizes = [13_093, 16_384, 32_768, 19_675];
		const chunks = chunkSizes.map((size, index) => {
			const chunk = new Uint8Array(size);
			chunk.fill(index + 1);
			return chunk;
		});
		const request = nativeRegistryTestInternals.buildRequest({
			method: "POST",
			uri: "/upload",
			bodyStream: {
				async read() {
					return chunks.shift() ?? null;
				},
				async cancel() {},
			},
		});

		const reader = request.body?.getReader();
		expect(reader).toBeDefined();

		const sizes: number[] = [];
		let totalBytes = 0;
		for (;;) {
			const next = await reader!.read();
			if (next.done) break;
			sizes.push(next.value.byteLength);
			totalBytes += next.value.byteLength;
		}

		expect(sizes).toEqual(chunkSizes);
		expect(totalBytes).toBe(80 * 1024);
	});

	test("streams multi-chunk responses through the native body stream", async () => {
		const writes: Uint8Array[] = [];
		let finish!: () => void;
		const finished = new Promise<void>((resolve) => {
			finish = resolve;
		});

		const responseBodyStream = {
			async write(chunk: Uint8Array) {
				writes.push(new Uint8Array(chunk));
			},
			async end() {
				finish();
			},
			async error(message: string) {
				throw new Error(message);
			},
		};
		const largeChunk = new Uint8Array(64 * 1024 + 1);
		largeChunk.fill(7);
		const response = new Response(
			new ReadableStream<Uint8Array>({
				start(controller) {
					controller.enqueue(new Uint8Array([1]));
					controller.enqueue(largeChunk);
					controller.close();
				},
			}),
			{
				status: 202,
				headers: {
					"x-streamed": "yes",
				},
			},
		);

		const runtimeResponse =
			await nativeRegistryTestInternals.toRuntimeHttpResponse(
				response,
				responseBodyStream,
			);
		await finished;

		expect(runtimeResponse).toEqual({
			status: 202,
			headers: {
				"x-streamed": "yes",
			},
			stream: true,
		});
		expect(writes.map((chunk) => chunk.byteLength)).toEqual([
			1,
			64 * 1024,
			1,
		]);
		expect(writes[0]).toEqual(new Uint8Array([1]));
		expect(writes[1][0]).toBe(7);
		expect(writes[2][0]).toBe(7);
	});
});
