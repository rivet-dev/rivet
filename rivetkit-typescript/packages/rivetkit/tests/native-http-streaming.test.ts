import { describe, expect, test } from "vitest";
import { nativeRegistryTestInternals } from "../src/registry/native";

describe("native http response streaming", () => {
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
