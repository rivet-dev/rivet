import { describe, expect, it, vi } from "vitest";
import { performance } from "node:perf_hooks";
import { pack, unpack } from "fdb-tuple";
import { createTraces } from "../src/index.js";
import type { TracesDriver } from "../src/types.js";
import { CHUNK_VERSIONED } from "../schemas/versioned.js";

class InMemoryTracesDriver implements TracesDriver {
	private store = new Map<string, Uint8Array>();

	async get(key: Uint8Array): Promise<Uint8Array | null> {
		const value = this.store.get(toKey(key));
		return value ? new Uint8Array(value) : null;
	}

	async set(key: Uint8Array, value: Uint8Array): Promise<void> {
		this.store.set(toKey(key), new Uint8Array(value));
	}

	async delete(key: Uint8Array): Promise<void> {
		this.store.delete(toKey(key));
	}

	async deletePrefix(prefix: Uint8Array): Promise<void> {
		const prefixBuf = Buffer.from(prefix);
		for (const key of this.store.keys()) {
			const keyBuf = Buffer.from(key, "hex");
			if (hasPrefix(keyBuf, prefixBuf)) {
				this.store.delete(key);
			}
		}
	}

	async list(prefix: Uint8Array): Promise<Array<{ key: Uint8Array; value: Uint8Array }>> {
		const prefixBuf = Buffer.from(prefix);
		const entries: Array<{ key: Uint8Array; value: Uint8Array }> = [];
		for (const [key, value] of this.store.entries()) {
			const keyBuf = Buffer.from(key, "hex");
			if (hasPrefix(keyBuf, prefixBuf)) {
				entries.push({ key: new Uint8Array(keyBuf), value: new Uint8Array(value) });
			}
		}
		entries.sort((a, b) => Buffer.compare(a.key, b.key));
		return entries;
	}

	async listRange(
		start: Uint8Array,
		end: Uint8Array,
		options?: { reverse?: boolean; limit?: number },
	): Promise<Array<{ key: Uint8Array; value: Uint8Array }>> {
		const startBuf = Buffer.from(start);
		const endBuf = Buffer.from(end);
		const entries: Array<{ key: Uint8Array; value: Uint8Array }> = [];

		for (const [key, value] of this.store.entries()) {
			const keyBuf = Buffer.from(key, "hex");
			if (Buffer.compare(keyBuf, startBuf) < 0) {
				continue;
			}
			if (Buffer.compare(keyBuf, endBuf) >= 0) {
				continue;
			}
			entries.push({ key: new Uint8Array(keyBuf), value: new Uint8Array(value) });
		}

		entries.sort((a, b) => Buffer.compare(a.key, b.key));
		if (options?.reverse) {
			entries.reverse();
		}
		if (options?.limit !== undefined) {
			return entries.slice(0, options.limit);
		}
		return entries;
	}

	async batch(writes: Array<{ key: Uint8Array; value: Uint8Array }>): Promise<void> {
		for (const write of writes) {
			this.store.set(toKey(write.key), new Uint8Array(write.value));
		}
	}

	entries(): Array<{ key: Uint8Array; value: Uint8Array }> {
		const output: Array<{ key: Uint8Array; value: Uint8Array }> = [];
		for (const [key, value] of this.store.entries()) {
			output.push({
				key: new Uint8Array(Buffer.from(key, "hex")),
				value: new Uint8Array(value),
			});
		}
		output.sort((a, b) => Buffer.compare(a.key, b.key));
		return output;
	}
}

class DelayedTracesDriver extends InMemoryTracesDriver {
	private blocker: Promise<void>;
	private resolveBlocker: (() => void) | null = null;

	constructor() {
		super();
		this.blocker = new Promise((resolve) => {
			this.resolveBlocker = resolve;
		});
	}

	async set(key: Uint8Array, value: Uint8Array): Promise<void> {
		await this.blocker;
		return super.set(key, value);
	}

	releaseWrites(): void {
		this.resolveBlocker?.();
		this.resolveBlocker = null;
	}
}

function toKey(key: Uint8Array): string {
	return Buffer.from(key).toString("hex");
}

function hasPrefix(key: Uint8Array, prefix: Uint8Array): boolean {
	if (prefix.length > key.length) {
		return false;
	}
	for (let i = 0; i < prefix.length; i++) {
		if (key[i] !== prefix[i]) {
			return false;
		}
	}
	return true;
}

type FakeClock = {
	nowUnixMs: () => number;
	nowMonoMs: () => number;
	set: (unixMs: number, monoMs?: number) => void;
	advance: (ms: number) => void;
	restore: () => void;
};

function installFakeClock(initialUnixMs = 1_700_000_000_000): FakeClock {
	let unixMs = initialUnixMs;
	let monoMs = 0;
	const dateSpy = vi.spyOn(Date, "now").mockImplementation(() => unixMs);
	const perfSpy = vi.spyOn(performance, "now").mockImplementation(() => monoMs);

	return {
		nowUnixMs: () => unixMs,
		nowMonoMs: () => monoMs,
		set: (nextUnixMs: number, nextMonoMs = monoMs) => {
			unixMs = nextUnixMs;
			monoMs = nextMonoMs;
		},
		advance: (ms: number) => {
			unixMs += ms;
			monoMs += ms;
		},
		restore: () => {
			dateSpy.mockRestore();
			perfSpy.mockRestore();
		},
	};
}

describe("traces", () => {
	it("exports spans with events, attributes, and status", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({
				driver,
				resource: {
					attributes: [{ key: "service.name", value: { stringValue: "test" } }],
				},
			});

			const root = traces.startSpan("root", { attributes: { foo: "bar" } });
			traces.setAttributes(root, { count: 2 });
			traces.emitEvent(root, "evt", { attributes: { ok: true } });
			traces.setStatus(root, { code: "OK" });
			traces.endSpan(root, { status: { code: "OK" } });

			const now = clock.nowUnixMs();
			const result = await traces.readRange({
				startMs: now - 60_000,
				endMs: now + 60_000,
				limit: 100,
			});

			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans).toHaveLength(1);
			const span = spans[0];
			expect(span.name).toBe("root");
			expect(span.status?.code).toBe(1);
			expect(span.events?.[0].name).toBe("evt");

			const attrMap = new Map(
				span.attributes?.map((attr) => [attr.key, attr.value]),
			);
			expect(attrMap.get("foo")?.stringValue).toBe("bar");
			expect(attrMap.get("count")?.intValue).toBe("2");
		} finally {
			clock.restore();
		}
	});

	it("propagates parent span via withSpan", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver });

			const root = traces.startSpan("root");
			let child: ReturnType<typeof traces.startSpan> | null = null;
			traces.withSpan(root, () => {
				child = traces.startSpan("child");
				traces.endSpan(child);
			});
			traces.endSpan(root);

			const now = clock.nowUnixMs();
			const result = await traces.readRange({
				startMs: now - 60_000,
				endMs: now + 60_000,
				limit: 100,
			});
			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			const childSpan = spans.find((span) => span.name === "child");
			const rootSpan = spans.find((span) => span.name === "root");
			expect(childSpan).toBeDefined();
			expect(rootSpan).toBeDefined();
			expect(childSpan?.parentSpanId).toBe(rootSpan?.spanId);
		} finally {
			clock.restore();
		}
	});

	it("splits chunks and stores keys in order", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({
				driver,
				targetChunkBytes: 200,
				maxChunkBytes: 1024,
				maxChunkAgeMs: 60_000,
			});

			const span = traces.startSpan("chunked");
			for (let i = 0; i < 40; i++) {
				traces.emitEvent(span, `e${i}`, { attributes: { idx: i } });
			}
			traces.endSpan(span);
			await traces.flush();

			const entries = driver.entries();
			expect(entries.length).toBeGreaterThan(1);

			const decoded = entries.map((entry) =>
				unpack(Buffer.from(entry.key)) as [number, number, number],
			);
			for (const tuple of decoded) {
				expect(tuple[0]).toBe(1);
			}
			for (let i = 1; i < decoded.length; i++) {
				const prev = decoded[i - 1];
				const curr = decoded[i];
				const inOrder =
					curr[1] > prev[1] || (curr[1] === prev[1] && curr[2] >= prev[2]);
				expect(inOrder).toBe(true);
			}
		} finally {
			clock.restore();
		}
	});

	it("creates snapshots after threshold", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({
				driver,
				snapshotBytesThreshold: 1,
				snapshotIntervalMs: 60_000,
			});

			const span = traces.startSpan("snap");
			traces.emitEvent(span, "evt");
			await traces.flush();

			const entry = driver.entries()[0];
			const chunk = CHUNK_VERSIONED.deserializeWithEmbeddedVersion(entry.value);
			const hasSnapshot = chunk.records.some(
				(record) => record.body.tag === "SpanSnapshot",
			);
			expect(hasSnapshot).toBe(true);
		} finally {
			clock.restore();
		}
	});

	it("hydrates spans across chunks with previous snapshot", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({
				driver,
				targetChunkBytes: 300,
				maxChunkBytes: 1024,
				maxChunkAgeMs: 60_000,
			});

			const t0 = clock.nowUnixMs();
			const span = traces.startSpan("long");
			traces.emitEvent(span, "early", { timeUnixMs: t0 + 100 });
			await traces.flush();
			traces.emitEvent(span, "late", { timeUnixMs: t0 + 5000 });
			traces.endSpan(span, { status: { code: "OK" } });

			const result = await traces.readRange({
				startMs: t0 + 2000,
				endMs: t0 + 6000,
				limit: 10,
			});
			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans).toHaveLength(1);
			expect(spans[0].name).toBe("long");
			expect(spans[0].events?.[0].name).toBe("late");
		} finally {
			clock.restore();
		}
	});

	it("clamps limit by span count", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver, maxReadLimit: 2 });

			const a = traces.startSpan("a");
			const b = traces.startSpan("b");
			const c = traces.startSpan("c");
			traces.endSpan(a);
			traces.endSpan(b);
			traces.endSpan(c);

			const now = clock.nowUnixMs();
			const result = await traces.readRange({
				startMs: now - 60_000,
				endMs: now + 60_000,
				limit: 10,
			});

			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans.length).toBe(2);
			expect(result.clamped).toBe(true);
		} finally {
			clock.restore();
		}
	});

	it("drops deep spans when maxActiveSpans exceeded", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver, maxActiveSpans: 1 });

			const root = traces.startSpan("root");
			const child = traces.startSpan("child", { parent: root });

			expect(root.isActive()).toBe(true);
			expect(child.isActive()).toBe(false);
			expect(() => traces.emitEvent(child, "nope")).toThrow();
			traces.endSpan(root);
		} finally {
			clock.restore();
		}
	});

	it("reads from the current chunk without a flush", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver });
			const span = traces.startSpan("live");
			traces.emitEvent(span, "evt");
			traces.endSpan(span);

			const now = clock.nowUnixMs();
			const result = await traces.readRange({
				startMs: now - 1000,
				endMs: now + 1000,
				limit: 10,
			});

			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans).toHaveLength(1);
			expect(spans[0].events?.[0].name).toBe("evt");
		} finally {
			clock.restore();
		}
	});

	it("reads pending chunks before async writes complete", async () => {
		const clock = installFakeClock();
		const driver = new DelayedTracesDriver();
		try {
			const traces = createTraces({ driver });
			const span = traces.startSpan("pending");
			traces.emitEvent(span, "evt");
			traces.endSpan(span);

			const flushPromise = traces.flush();
			const now = clock.nowUnixMs();
			const result = await traces.readRange({
				startMs: now - 1000,
				endMs: now + 1000,
				limit: 10,
			});
			driver.releaseWrites();
			await flushPromise;

			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans).toHaveLength(1);
			expect(spans[0].name).toBe("pending");
		} finally {
			clock.restore();
		}
	});

	it("hydrates long spans across bucket boundaries with correct start time", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({
				driver,
				bucketSizeSec: 2,
				targetChunkBytes: 200,
				maxChunkBytes: 1024,
				maxChunkAgeMs: 60_000,
				snapshotBytesThreshold: 999_999,
			});

			const t0 = clock.nowUnixMs();
			const span = traces.startSpan("long");
			traces.emitEvent(span, "e1", { timeUnixMs: t0 + 100 });
			await traces.flush();

			clock.advance(2500);
			const t1 = clock.nowUnixMs();
			traces.emitEvent(span, "e2", { timeUnixMs: t1 + 10 });

			clock.advance(500);
			const t2 = clock.nowUnixMs();
			traces.endSpan(span);
			await traces.flush();

			const result = await traces.readRange({
				startMs: t1,
				endMs: t2 + 100,
				limit: 10,
			});

			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans).toHaveLength(1);
			expect(spans[0].events?.map((evt) => evt.name)).toEqual(["e2"]);
			expect(spans[0].startTimeUnixNano).toBe(
				(BigInt(t0) * 1_000_000n).toString(),
			);
		} finally {
			clock.restore();
		}
	});

	it("applies snapshot updates when updates fall outside the read range", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({
				driver,
				bucketSizeSec: 2,
				targetChunkBytes: 200,
				maxChunkBytes: 1024,
				snapshotBytesThreshold: 1,
				snapshotIntervalMs: 0,
			});

			const span = traces.startSpan("snap");
			traces.setAttributes(span, { foo: "bar" });
			await traces.flush();

			clock.advance(2500);
			const t1 = clock.nowUnixMs();
			traces.emitEvent(span, "later", { timeUnixMs: t1 });
			traces.endSpan(span);
			await traces.flush();

			const result = await traces.readRange({
				startMs: t1 - 1,
				endMs: t1 + 1000,
				limit: 10,
			});

			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			const spanOut = spans[0];
			const attrMap = new Map(
				spanOut.attributes?.map((attr) => [attr.key, attr.value]),
			);
			expect(attrMap.get("foo")?.stringValue).toBe("bar");
		} finally {
			clock.restore();
		}
	});

	it("treats start as inclusive and end as exclusive", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver });
			const t0 = clock.nowUnixMs();
			const span = traces.startSpan("bounds");
			traces.emitEvent(span, "start", { timeUnixMs: t0 });
			traces.emitEvent(span, "end", { timeUnixMs: t0 + 10 });

			const result = await traces.readRange({
				startMs: t0,
				endMs: t0 + 10,
				limit: 10,
			});

			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans).toHaveLength(1);
			expect(spans[0].events?.map((evt) => evt.name)).toEqual(["start"]);
		} finally {
			clock.restore();
		}
	});

	it("marks clamped when user limit is reached", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver, maxReadLimit: 100 });
			const a = traces.startSpan("a");
			const b = traces.startSpan("b");
			traces.endSpan(a);
			traces.endSpan(b);

			const now = clock.nowUnixMs();
			const result = await traces.readRange({
				startMs: now - 1000,
				endMs: now + 1000,
				limit: 1,
			});

			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans).toHaveLength(1);
			expect(result.clamped).toBe(true);
		} finally {
			clock.restore();
		}
	});

	it("forces chunk rollover by age", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({
				driver,
				targetChunkBytes: 1024,
				maxChunkBytes: 1024 * 1024,
				maxChunkAgeMs: 10,
			});

			const span = traces.startSpan("aged");
			traces.emitEvent(span, "e1");
			clock.advance(11);
			traces.emitEvent(span, "e2");
			traces.endSpan(span);
			await traces.flush();

			const entries = driver.entries();
			expect(entries.length).toBeGreaterThan(1);
		} finally {
			clock.restore();
		}
	});

	it("rejects updates after a span ends", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver });
			const span = traces.startSpan("ended");
			traces.endSpan(span);
			expect(() => traces.emitEvent(span, "evt")).toThrow();
			expect(() => traces.setAttributes(span, { foo: "bar" })).toThrow();
		} finally {
			clock.restore();
		}
	});

	it("returns empty export for invalid ranges and limits", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver });
			const span = traces.startSpan("ignored");
			traces.endSpan(span);

			const now = clock.nowUnixMs();
			const zeroLimit = await traces.readRange({
				startMs: now - 1000,
				endMs: now + 1000,
				limit: 0,
			});
			expect(
				zeroLimit.otlp.resourceSpans[0].scopeSpans[0].spans,
			).toHaveLength(0);

			const inverted = await traces.readRange({
				startMs: now + 1000,
				endMs: now,
				limit: 10,
			});
			expect(
				inverted.otlp.resourceSpans[0].scopeSpans[0].spans,
			).toHaveLength(0);
		} finally {
			clock.restore();
		}
	});

	it("skips corrupted chunks when hydrating long spans", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({
				driver,
				bucketSizeSec: 2,
				targetChunkBytes: 200,
				maxChunkBytes: 1024,
				snapshotBytesThreshold: 999_999,
			});

			const t0 = clock.nowUnixMs();
			const span = traces.startSpan("long");
			traces.emitEvent(span, "early", { timeUnixMs: t0 + 100 });
			await traces.flush();

			const laterBucketSec = Math.floor((t0 + 2500) / 1000 / 2) * 2;
			const corruptKey = pack([1, laterBucketSec, 0]);
			await driver.set(corruptKey, new Uint8Array([0, 1, 2, 3]));

			traces.emitEvent(span, "late", { timeUnixMs: t0 + 2600 });
			traces.endSpan(span);

			const result = await traces.readRange({
				startMs: t0 + 2500,
				endMs: t0 + 3000,
				limit: 10,
			});
			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans).toHaveLength(1);
			expect(spans[0].events?.[0].name).toBe("late");
		} finally {
			clock.restore();
		}
	});

	it("throws when a single record exceeds maxChunkBytes", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({
				driver,
				targetChunkBytes: 128,
				maxChunkBytes: 128,
			});
			const bigValue = "x".repeat(10_000);
			expect(() =>
				traces.startSpan("big", { attributes: { big: bigValue } }),
			).toThrow();
		} finally {
			clock.restore();
		}
	});

	it("handles the clock moving backwards without throwing", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver });
			const t0 = clock.nowUnixMs();
			const span = traces.startSpan("time");
			traces.emitEvent(span, "forward", { timeUnixMs: t0 + 100 });
			clock.advance(-5000);
			traces.emitEvent(span, "back");
			traces.endSpan(span);

			const result = await traces.readRange({
				startMs: t0 - 6000,
				endMs: t0 + 2000,
				limit: 10,
			});

			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans).toHaveLength(1);
			expect(spans[0].events?.map((evt) => evt.name)).toEqual(["forward"]);
		} finally {
			clock.restore();
		}
	});

	it("rejects ending a span twice", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver });
			const span = traces.startSpan("double-end");
			traces.endSpan(span);
			expect(() => traces.endSpan(span)).toThrow();
		} finally {
			clock.restore();
		}
	});

	it("returns no spans when the range has no records", async () => {
		const clock = installFakeClock();
		const driver = new InMemoryTracesDriver();
		try {
			const traces = createTraces({ driver });
			const t0 = clock.nowUnixMs();
			const span = traces.startSpan("quiet");
			traces.endSpan(span);

			const result = await traces.readRange({
				startMs: t0 + 10_000,
				endMs: t0 + 20_000,
				limit: 10,
			});

			const spans = result.otlp.resourceSpans[0].scopeSpans[0].spans;
			expect(spans).toHaveLength(0);
		} finally {
			clock.restore();
		}
	});
});
