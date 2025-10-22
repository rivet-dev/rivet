import { setupTest } from "rivetkit/test";
import { expect, test, vi } from "vitest";
import { registry } from "../src/backend/registry";

// Mock Yjs to avoid complex binary operations in tests
vi.mock("yjs", () => ({
	Doc: vi.fn().mockImplementation(() => ({
		getText: vi.fn().mockReturnValue({
			toString: vi.fn().mockReturnValue(""),
			observe: vi.fn(),
			delete: vi.fn(),
			insert: vi.fn(),
		}),
		transact: vi.fn((fn) => fn()),
		destroy: vi.fn(),
	})),
	applyUpdate: vi.fn(),
	encodeStateAsUpdate: vi.fn().mockReturnValue(new Uint8Array([1, 2, 3, 4])),
}));

test("CRDT document can handle initial state", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const doc = client.yjsDocument.getOrCreate(["test-doc"]);

	// Test initial state
	const state = await doc.getState();
	expect(state.docData).toBeInstanceOf(Uint8Array);
	expect(state.docData.length).toBe(0);
	expect(state.lastModified).toBe(0);
});

test("CRDT document can apply updates", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const doc = client.yjsDocument.getOrCreate(["test-updates"]);

	// Mock update data as binary
	const updateBytes = new Uint8Array([1, 2, 3, 4, 5]);

	// Apply an update
	await doc.applyUpdate(updateBytes);

	// Verify state was updated
	const state = await doc.getState();
	expect(state.docData.length).toBeGreaterThan(0);
	expect(state.lastModified).toBeGreaterThan(0);
});

test("CRDT document handles multiple updates", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const doc = client.yjsDocument.getOrCreate(["test-multiple"]);

	const update1 = new Uint8Array([1, 2, 3]);
	const update2 = new Uint8Array([4, 5, 6]);
	const update3 = new Uint8Array([7, 8, 9]);

	// Apply multiple updates
	await doc.applyUpdate(update1);
	const state1 = await doc.getState();
	const firstModified = state1.lastModified;

	await doc.applyUpdate(update2);
	const state2 = await doc.getState();
	const secondModified = state2.lastModified;

	await doc.applyUpdate(update3);
	const state3 = await doc.getState();
	const thirdModified = state3.lastModified;

	// Verify timestamps are increasing
	expect(secondModified).toBeGreaterThanOrEqual(firstModified);
	expect(thirdModified).toBeGreaterThanOrEqual(secondModified);

	// Verify state is updated
	expect(state3.docData.length).toBeGreaterThan(0);
	expect(state3.lastModified).toBe(thirdModified);
});

test("CRDT document handles binary data correctly", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const doc = client.yjsDocument.getOrCreate(["test-encoding"]);

	// Test with specific binary data
	const updateBytes = new TextEncoder().encode("Hello, collaborative world!");

	await doc.applyUpdate(updateBytes);

	const state = await doc.getState();
	expect(state.docData.length).toBeGreaterThan(0);
	expect(state.lastModified).toBeGreaterThan(0);
});
