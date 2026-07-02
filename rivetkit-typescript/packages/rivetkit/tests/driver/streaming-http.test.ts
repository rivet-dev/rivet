import { describe, expect, test } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest } from "./shared-utils";

function delay(ms: number): Promise<"timeout"> {
	return new Promise((resolve) => setTimeout(() => resolve("timeout"), ms));
}

describeDriverMatrix(
	"Streaming Http",
	(driverTestConfig) => {
		describe("streaming http", () => {
			test("streams response chunks before the body completes", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.rawHttpActor.getOrCreate([
					"stream-response",
				]);

				const response = await actor.fetch("api/stream");
				expect(response.ok).toBe(true);
				expect(response.headers.get("content-type")).toContain(
					"text/event-stream",
				);

				const reader = response.body?.getReader();
				expect(reader).toBeDefined();
				const decoder = new TextDecoder();
				const first = await reader!.read();
				expect(first.done).toBe(false);
				expect(decoder.decode(first.value)).toBe("data: first\n\n");

				const secondRead = reader!.read();
				const earlySecond = await Promise.race([
					secondRead,
					delay(50),
				]);
				expect(earlySecond).toBe("timeout");

				const second = await secondRead;
				expect(second.done).toBe(false);
				expect(decoder.decode(second.value)).toBe("data: second\n\n");
				expect(await reader!.read()).toEqual({
					done: true,
					value: undefined,
				});
			});

			test("exposes gateway-chunked request bodies as Request streams", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.rawHttpActor.getOrCreate([
					"stream-upload",
				]);
				const requestBody = new Uint8Array(80 * 1024);
				requestBody.fill(1, 0, 40 * 1024);
				requestBody.fill(2, 40 * 1024);

				const response = await actor.fetch("api/upload-stream", {
					method: "POST",
					body: Buffer.from(requestBody),
				});

				expect(response.ok).toBe(true);
				const body = (await response.json()) as {
					chunkCount: number;
					contentLength: string | null;
					sizes: number[];
					totalBytes: number;
				};
				expect(body.totalBytes, JSON.stringify(body)).toBe(requestBody.byteLength);
				expect(body.chunkCount).toBeGreaterThanOrEqual(2);
				expect(Math.max(...body.sizes)).toBeLessThanOrEqual(64 * 1024);
			});
		});
	},
	{
		runtimes: ["native"],
		encodings: ["bare"],
		sqliteBackends: ["remote"],
		config: {
			useRealTimers: true,
		},
	},
);
