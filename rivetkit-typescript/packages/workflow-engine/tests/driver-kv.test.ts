import { beforeEach, describe, expect, it } from "vitest";
import {
	buildMessageKey,
	buildMessagePrefix,
	buildWorkflowStateKey,
	parseMessageKey,
} from "../src/keys.js";
import { InMemoryDriver } from "../src/testing.js";

const modes = ["yield", "live"] as const;

const encoder = new TextEncoder();

function encode(value: string): Uint8Array {
	return encoder.encode(value);
}

for (const mode of modes) {
	describe(
		`Workflow Engine Driver KV (${mode})`,
		{ sequential: true },
		() => {
			let driver: InMemoryDriver;

			beforeEach(() => {
				driver = new InMemoryDriver();
				driver.latency = 0;
			});

			it("should set and get values", async () => {
				const key = encode("key-a");
				const value = encode("value-a");

				await driver.set(key, value);

				const result = await driver.get(key);
				expect(result).toEqual(value);
			});

			it("should return null for missing keys", async () => {
				const result = await driver.get(encode("missing"));
				expect(result).toBeNull();
			});

			it("should overwrite existing keys", async () => {
				const key = encode("key-b");
				await driver.set(key, encode("first"));
				await driver.set(key, encode("second"));

				const result = await driver.get(key);
				expect(result).toEqual(encode("second"));
			});

			it("should delete keys", async () => {
				const key = encode("key-c");
				await driver.set(key, encode("value"));
				await driver.delete(key);

				const result = await driver.get(key);
				expect(result).toBeNull();
			});

			it("should list keys by prefix", async () => {
				await driver.set(buildMessageKey("a"), encode("one"));
				await driver.set(buildMessageKey("b"), encode("two"));
				await driver.set(buildWorkflowStateKey(), encode("state"));

				const entries = await driver.list(buildMessagePrefix());
				const ids = entries.map((entry) => parseMessageKey(entry.key));

				expect(ids).toEqual(["a", "b"]);
			});

			it("should delete only keys with a prefix", async () => {
				const messageKey = buildMessageKey("message");
				const stateKey = buildWorkflowStateKey();

				await driver.set(messageKey, encode("message"));
				await driver.set(stateKey, encode("state"));

				await driver.deletePrefix(buildMessagePrefix());

				expect(await driver.get(messageKey)).toBeNull();
				expect(await driver.get(stateKey)).not.toBeNull();
			});

			it("should list messages in sorted order", async () => {
				await driver.set(buildMessageKey("b"), encode("two"));
				await driver.set(buildMessageKey("a"), encode("one"));

				const entries = await driver.list(buildMessagePrefix());
				const ids = entries.map((entry) => parseMessageKey(entry.key));

				expect(ids).toEqual(["a", "b"]);
			});

			it("should batch writes", async () => {
				const keyA = encode("batch-a");
				const keyB = encode("batch-b");

				await driver.batch([
					{ key: keyA, value: encode("one") },
					{ key: keyB, value: encode("two") },
				]);

				expect(await driver.get(keyA)).toEqual(encode("one"));
				expect(await driver.get(keyB)).toEqual(encode("two"));
			});

			it("should batch overwrite existing keys", async () => {
				const key = encode("batch-c");
				await driver.set(key, encode("old"));

				await driver.batch([{ key, value: encode("new") }]);

				expect(await driver.get(key)).toEqual(encode("new"));
			});
		},
	);
}
