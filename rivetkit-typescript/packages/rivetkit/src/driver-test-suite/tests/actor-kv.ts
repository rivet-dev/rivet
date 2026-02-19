import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";
import { describe, expect, test, type TestContext } from "vitest";

export function runActorKvTests(driverTestConfig: DriverTestConfig) {
	type KvTextHandle = {
		putText: (key: string, value: string) => Promise<unknown>;
		getText: (key: string) => Promise<string | null>;
		listText: (prefix: string) => Promise<Array<{ key: string; value: string }>>;
	};

	type KvArrayBufferHandle = {
		roundtripArrayBuffer: (key: string, bytes: number[]) => Promise<number[]>;
	};

		describe("Actor KV Tests", () => {
			test("supports text encoding and decoding", async (c: TestContext) => {
				const { client: rawClient } = await setupDriverTest(
					c,
					driverTestConfig,
				);
				const client = rawClient as any;
				const kvHandle =
					client.kvActor.getOrCreate(["kv-text"]) as unknown as KvTextHandle;

			await kvHandle.putText("greeting", "hello");
			const value = await kvHandle.getText("greeting");
			expect(value).toBe("hello");

			await kvHandle.putText("prefix-a", "alpha");
			await kvHandle.putText("prefix-b", "beta");

			const results = await kvHandle.listText("prefix-");
			const sorted = results.sort((a, b) => a.key.localeCompare(b.key));
			expect(sorted).toEqual([
				{ key: "prefix-a", value: "alpha" },
				{ key: "prefix-b", value: "beta" },
			]);
		});

		test(
			"supports arrayBuffer encoding and decoding",
			async (c: TestContext) => {
				const { client: rawClient } = await setupDriverTest(
					c,
					driverTestConfig,
				);
				const client = rawClient as any;
				const kvHandle = client.kvActor.getOrCreate([
					"kv-array-buffer",
				]) as unknown as KvArrayBufferHandle;

				const values = await kvHandle.roundtripArrayBuffer("bytes", [
					4,
					8,
					15,
					16,
					23,
					42,
				]);
				expect(values).toEqual([4, 8, 15, 16, 23, 42]);
			},
			);
		});
	}
