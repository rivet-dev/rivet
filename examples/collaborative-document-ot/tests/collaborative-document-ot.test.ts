import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/index.ts";
import { apply, diff, type Operation, transform } from "../src/ot.ts";

describe("transform", () => {
	test("concurrent inserts at the same position converge", () => {
		const base = "hello";
		const a: Operation = [{ retain: 5 }, { insert: " world" }];
		const b: Operation = [{ retain: 5 }, { insert: "!" }];
		const [a2, b2] = transform(a, b);
		expect(apply(apply(base, a), b2)).toBe("hello world!");
		expect(apply(apply(base, b), a2)).toBe("hello world!");
	});

	test("an insert inside a concurrent delete survives", () => {
		const base = "abcdef";
		const del = diff(base, "af");
		const ins = diff(base, "abcXdef");
		const [del2, ins2] = transform(del, ins);
		expect(apply(apply(base, del), ins2)).toBe("aXf");
		expect(apply(apply(base, ins), del2)).toBe("aXf");
	});

	test("random concurrent edits converge", () => {
		const alphabet = "abcxyz ";
		const randomText = (length: number) =>
			Array.from(
				{ length },
				() => alphabet[Math.floor(Math.random() * alphabet.length)],
			).join("");
		const randomEdit = (text: string) => {
			const start = Math.floor(Math.random() * (text.length + 1));
			const end =
				start + Math.floor(Math.random() * (text.length - start + 1));
			return (
				text.slice(0, start) +
				randomText(Math.floor(Math.random() * 4)) +
				text.slice(end)
			);
		};

		for (let i = 0; i < 500; i++) {
			const base = randomText(Math.floor(Math.random() * 12));
			const a = diff(base, randomEdit(base));
			const b = diff(base, randomEdit(base));
			const [a2, b2] = transform(a, b);
			expect(apply(apply(base, a), b2)).toBe(apply(apply(base, b), a2));
		}
	});
});

describe("document actor", () => {
	test("transforms operations based on an old revision", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const doc = client.document.getOrCreate([
			`test-concurrent-${crypto.randomUUID()}`,
		]);

		await doc.submitOperation("setup", 0, [{ insert: "hello" }]);
		const base = await doc.getSnapshot();
		expect(base).toEqual({ text: "hello", revision: 1 });

		// Both clients edit revision 1 without seeing each other's edit.
		const [first, second] = await Promise.all([
			doc.submitOperation("alice", 1, diff("hello", "hello world")),
			doc.submitOperation("bob", 1, diff("hello", "Oh, hello")),
		]);
		expect([first, second].sort()).toEqual([2, 3]);

		const snapshot = await doc.getSnapshot();
		expect(snapshot).toEqual({ text: "Oh, hello world", revision: 3 });
	});

	test("rejects a revision from the future", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const doc = client.document.getOrCreate([
			`test-future-${crypto.randomUUID()}`,
		]);
		await expect(
			doc.submitOperation("alice", 5, [{ insert: "x" }]),
		).rejects.toThrow("unknown base revision");
	});
});
