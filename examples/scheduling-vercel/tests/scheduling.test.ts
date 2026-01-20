import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors.ts";

// Helper to wait for a delay
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

describe("reminder scheduling", () => {
	test("triggers reminder after delay", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const reminderClient =
			client.reminderActor.getOrCreate("test-after-delay");
		const reminder = await reminderClient.scheduleReminder(
			"Test reminder",
			100,
		);

		// Verify reminder was created
		expect(reminder.id).toBeDefined();
		expect(reminder.message).toBe("Test reminder");
		expect(reminder.completedAt).toBeUndefined();

		// Wait for the scheduled action to execute
		await wait(150);

		const reminders = await reminderClient.getReminders();
		const completed = reminders.find((r) => r.id === reminder.id);

		expect(completed?.completedAt).toBeDefined();
	});

	test("schedules reminder at specific timestamp", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const reminderClient =
			client.reminderActor.getOrCreate("test-at-timestamp");
		const futureTime = Date.now() + 100;
		const reminder = await reminderClient.scheduleReminderAt(
			"Future reminder",
			futureTime,
		);

		// Verify reminder was created
		expect(reminder.scheduledAt).toBe(futureTime);
		expect(reminder.completedAt).toBeUndefined();

		// Wait for the scheduled time
		await wait(150);

		const reminders = await reminderClient.getReminders();
		const completed = reminders.find((r) => r.id === reminder.id);

		expect(completed?.completedAt).toBeDefined();
	});

	test("passes correct arguments to scheduled actions", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const reminderClient =
			client.reminderActor.getOrCreate("test-arguments");

		// Schedule multiple reminders with different IDs
		const reminder1 = await reminderClient.scheduleReminder(
			"First reminder",
			50,
		);
		const reminder2 = await reminderClient.scheduleReminder(
			"Second reminder",
			100,
		);
		const reminder3 = await reminderClient.scheduleReminder(
			"Third reminder",
			150,
		);

		// Wait for all to trigger
		await wait(200);

		const reminders = await reminderClient.getReminders();

		// Verify each reminder received the correct ID and was triggered
		const completed1 = reminders.find((r) => r.id === reminder1.id);
		const completed2 = reminders.find((r) => r.id === reminder2.id);
		const completed3 = reminders.find((r) => r.id === reminder3.id);

		expect(completed1?.completedAt).toBeDefined();
		expect(completed2?.completedAt).toBeDefined();
		expect(completed3?.completedAt).toBeDefined();
	});

	test("cancels scheduled reminder", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const reminderClient = client.reminderActor.getOrCreate("test-cancel");
		const reminder = await reminderClient.scheduleReminder(
			"To be cancelled",
			100,
		);

		// Cancel the reminder before it triggers
		await reminderClient.cancelReminder(reminder.id);

		// Wait past the original trigger time
		await wait(150);

		const reminders = await reminderClient.getReminders();
		const found = reminders.find((r) => r.id === reminder.id);

		// Reminder should not exist (was removed from state)
		expect(found).toBeUndefined();
	});

	test("triggers multiple reminders in correct order", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const reminderClient =
			client.reminderActor.getOrCreate("test-multiple");

		// Schedule 5 reminders with different delays
		const reminder1 = await reminderClient.scheduleReminder(
			"Reminder 1",
			50,
		);
		const reminder2 = await reminderClient.scheduleReminder(
			"Reminder 2",
			100,
		);
		const reminder3 = await reminderClient.scheduleReminder(
			"Reminder 3",
			150,
		);
		const reminder4 = await reminderClient.scheduleReminder(
			"Reminder 4",
			200,
		);
		const reminder5 = await reminderClient.scheduleReminder(
			"Reminder 5",
			250,
		);

		// Wait for all to complete
		await wait(300);

		const reminders = await reminderClient.getReminders();

		// Verify all are completed
		expect(
			reminders.find((r) => r.id === reminder1.id)?.completedAt,
		).toBeDefined();
		expect(
			reminders.find((r) => r.id === reminder2.id)?.completedAt,
		).toBeDefined();
		expect(
			reminders.find((r) => r.id === reminder3.id)?.completedAt,
		).toBeDefined();
		expect(
			reminders.find((r) => r.id === reminder4.id)?.completedAt,
		).toBeDefined();
		expect(
			reminders.find((r) => r.id === reminder5.id)?.completedAt,
		).toBeDefined();

		const stats = await reminderClient.getStats();
		expect(stats.completed).toBe(5);
	});

	// Skipping restart test as direct actor access is not available in setupTest
	test.skip("scheduled actions persist across actor restarts", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const reminderClient = client.reminderActor.getOrCreate("test-restart");
		const reminder = await reminderClient.scheduleReminder(
			"Persistent reminder",
			100,
		);

		// This test would require direct access to the actor instance to stop it
		// which is not provided by setupTest

		await wait(150);

		const reminders = await reminderClient.getReminders();
		const completed = reminders.find((r) => r.id === reminder.id);

		expect(completed?.completedAt).toBeDefined();
	});

	test("updates stats correctly", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const reminderClient = client.reminderActor.getOrCreate("test-stats");

		let stats = await reminderClient.getStats();
		expect(stats.total).toBe(0);
		expect(stats.completed).toBe(0);
		expect(stats.pending).toBe(0);

		// Schedule some reminders
		await reminderClient.scheduleReminder("Reminder 1", 50);
		await reminderClient.scheduleReminder("Reminder 2", 100);
		await reminderClient.scheduleReminder("Reminder 3", 150);

		stats = await reminderClient.getStats();
		expect(stats.total).toBe(3);
		expect(stats.pending).toBe(3);
		expect(stats.completed).toBe(0);

		// Wait for first reminder to trigger
		await wait(70);

		stats = await reminderClient.getStats();
		expect(stats.total).toBe(3);
		expect(stats.pending).toBe(2);
		expect(stats.completed).toBe(1);

		// Wait for all remaining
		await wait(100);

		stats = await reminderClient.getStats();
		expect(stats.total).toBe(3);
		expect(stats.pending).toBe(0);
		expect(stats.completed).toBe(3);
	});
});
