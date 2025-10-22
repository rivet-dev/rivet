import { setupTest } from "rivetkit/test";
import { expect, test } from "vitest";
import { registry } from "../src/backend/registry";

test("Sync system can handle contact synchronization", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const sync = client.contacts.getOrCreate(["test-sync-new"]);

	// Initial state should be empty (or may have existing data)
	const initialContacts = await sync.getAllContacts();
	const initialCount = initialContacts.length;

	// Push some contacts
	const contacts = [
		{
			id: "1",
			name: "Alice Johnson",
			email: "alice@example.com",
			phone: "555-0001",
			updatedAt: Date.now() - 1000,
		},
		{
			id: "2",
			name: "Bob Smith",
			email: "bob@example.com",
			phone: "555-0002",
			updatedAt: Date.now(),
		},
	];

	const pushResult = await sync.pushChanges(contacts);
	expect(pushResult).toMatchObject({
		timestamp: expect.any(Number),
	});

	// Verify contacts were stored
	const allContacts = await sync.getAllContacts();
	expect(allContacts).toHaveLength(initialCount + 2);
	expect(allContacts).toEqual(expect.arrayContaining(contacts));
});

test("Sync system handles conflict resolution with last-write-wins", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const sync = client.contacts.getOrCreate(["test-conflicts"]);

	const oldTimestamp = Date.now() - 2000;
	const newTimestamp = Date.now();

	// Push initial contact
	const originalContact = {
		id: "conflict-test",
		name: "Original Name",
		email: "original@example.com",
		phone: "555-0000",
		updatedAt: oldTimestamp,
	};

	await sync.pushChanges([originalContact]);

	// Push conflicting update with newer timestamp
	const updatedContact = {
		id: "conflict-test",
		name: "Updated Name",
		email: "updated@example.com",
		phone: "555-1111",
		updatedAt: newTimestamp,
	};

	await sync.pushChanges([updatedContact]);

	// Verify newer version won
	const contacts = await sync.getAllContacts();
	const conflictContact = contacts.find((c) => c.id === "conflict-test");
	expect(conflictContact).toEqual(updatedContact);

	// Try to push older version - should be ignored
	const olderContact = {
		id: "conflict-test",
		name: "Older Name",
		email: "older@example.com",
		phone: "555-9999",
		updatedAt: oldTimestamp - 1000,
	};

	await sync.pushChanges([olderContact]);

	// Verify newer version is still there
	const finalContacts = await sync.getAllContacts();
	const finalContact = finalContacts.find((c) => c.id === "conflict-test");
	expect(finalContact).toEqual(updatedContact);
});

test("Sync system tracks changes after timestamp", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const sync = client.contacts.getOrCreate(["test-changes"]);

	const baseTime = Date.now();

	// Add some contacts at different times
	const contact1 = {
		id: "1",
		name: "First Contact",
		email: "first@example.com",
		phone: "555-0001",
		updatedAt: baseTime - 1000,
	};

	const contact2 = {
		id: "2",
		name: "Second Contact",
		email: "second@example.com",
		phone: "555-0002",
		updatedAt: baseTime + 1000,
	};

	await sync.pushChanges([contact1]);
	await sync.pushChanges([contact2]);

	// Get changes after base time - should only return contact2
	const changes = await sync.getChanges(baseTime);
	expect(changes.changes).toHaveLength(1);
	expect(changes.changes[0]).toEqual(contact2);
	expect(changes.timestamp).toBeGreaterThanOrEqual(baseTime);

	// Get all changes - should return both
	const allChanges = await sync.getChanges(0);
	expect(allChanges.changes).toHaveLength(2);
});

test("Sync system provides statistics", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const sync = client.contacts.getOrCreate(["test-stats-new"]);

	// Initial stats
	const initialStats = await sync.getSyncStats();
	expect(initialStats).toMatchObject({
		totalContacts: expect.any(Number),
		lastSyncTime: expect.any(Number),
		deletedContacts: expect.any(Number),
	});

	const initialTotal = initialStats.totalContacts;

	// Add some contacts
	const contacts = [
		{
			id: "1",
			name: "Contact 1",
			email: "c1@example.com",
			phone: "555-0001",
			updatedAt: Date.now(),
		},
		{
			id: "2",
			name: "Contact 2",
			email: "c2@example.com",
			phone: "555-0002",
			updatedAt: Date.now(),
		},
		{
			id: "3",
			name: "",
			email: "deleted@example.com",
			phone: "555-0003",
			updatedAt: Date.now(),
		}, // Deleted contact
	];

	await sync.pushChanges(contacts);

	const stats = await sync.getSyncStats();
	expect(stats.totalContacts).toBe(initialTotal + 2); // Only non-deleted contacts
	expect(stats.deletedContacts).toBeGreaterThanOrEqual(1);
	expect(stats.lastSyncTime).toBeGreaterThan(initialStats.lastSyncTime);
});

test("Sync system reset functionality", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const sync = client.contacts.getOrCreate(["test-reset"]);

	// Add some contacts
	const contacts = [
		{
			id: "1",
			name: "Contact 1",
			email: "c1@example.com",
			phone: "555-0001",
			updatedAt: Date.now(),
		},
		{
			id: "2",
			name: "Contact 2",
			email: "c2@example.com",
			phone: "555-0002",
			updatedAt: Date.now(),
		},
	];

	await sync.pushChanges(contacts);

	// Verify contacts exist
	let allContacts = await sync.getAllContacts();
	expect(allContacts).toHaveLength(2);

	// Reset the system
	const resetResult = await sync.reset();
	expect(resetResult).toMatchObject({
		timestamp: expect.any(Number),
	});

	// Verify contacts are gone
	allContacts = await sync.getAllContacts();
	expect(allContacts).toEqual([]);

	const stats = await sync.getSyncStats();
	expect(stats.totalContacts).toBe(0);
});
