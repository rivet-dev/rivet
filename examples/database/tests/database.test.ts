import { setupTest } from "rivetkit/test";
import { expect, test, vi } from "vitest";
import { registry } from "../src/backend/registry";

// Mock authentication
vi.mock("../src/backend/my-utils", () => ({
	authenticate: vi.fn().mockResolvedValue("user123"),
}));

test("Database notes can handle basic CRUD operations", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const notes = client.notes.getOrCreate(["test-notes"]);

	// Test initial empty state
	const initialNotes = await notes.getNotes();
	expect(initialNotes).toEqual([]);

	// Create a new note
	const newNote = await notes.updateNote({ content: "My first note" });
	expect(newNote).toMatchObject({
		id: expect.stringMatching(/^note-\d+$/),
		content: "My first note",
		updatedAt: expect.any(Number),
	});

	// Verify note was added
	const notesAfterAdd = await notes.getNotes();
	expect(notesAfterAdd).toHaveLength(1);
	expect(notesAfterAdd[0]).toEqual(newNote);
});

test("Database notes can update existing notes", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const notes = client.notes.getOrCreate(["test-update"]);

	// Create a note
	const originalNote = await notes.updateNote({
		content: "Original content",
	});
	const originalTime = originalNote.updatedAt;

	// Update the note
	const updatedNote = await notes.updateNote({
		id: originalNote.id,
		content: "Updated content",
	});

	expect(updatedNote).toMatchObject({
		id: originalNote.id,
		content: "Updated content",
		updatedAt: expect.any(Number),
	});
	expect(updatedNote.updatedAt).toBeGreaterThanOrEqual(originalTime);

	// Verify only one note exists
	const allNotes = await notes.getNotes();
	expect(allNotes).toHaveLength(1);
	expect(allNotes[0]).toEqual(updatedNote);
});

test("Database notes can delete notes", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const notes = client.notes.getOrCreate(["test-delete"]);

	// Create multiple notes
	const note1 = await notes.updateNote({ content: "Note 1" });
	const note2 = await notes.updateNote({ content: "Note 2" });
	const note3 = await notes.updateNote({ content: "Note 3" });

	// Verify all notes exist
	let allNotes = await notes.getNotes();
	expect(allNotes).toHaveLength(3);

	// Delete middle note
	const deleteResult = await notes.deleteNote({ id: note2.id });
	expect(deleteResult).toBe(true);

	// Verify note was deleted
	allNotes = await notes.getNotes();
	expect(allNotes).toHaveLength(2);
	expect(allNotes.map((n) => n.id)).toEqual([note1.id, note3.id]);

	// Try to delete non-existent note
	const deleteNonExistent = await notes.deleteNote({ id: "non-existent" });
	expect(deleteNonExistent).toBe(false);
});

test("Database notes maintains proper timestamps", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const notes = client.notes.getOrCreate(["test-timestamps"]);

	const note1 = await notes.updateNote({ content: "First note" });
	const note2 = await notes.updateNote({ content: "Second note" });
	const note3 = await notes.updateNote({ content: "Third note" });

	expect(note2.updatedAt).toBeGreaterThanOrEqual(note1.updatedAt);
	expect(note3.updatedAt).toBeGreaterThanOrEqual(note2.updatedAt);

	const allNotes = await notes.getNotes();
	for (let i = 1; i < allNotes.length; i++) {
		expect(allNotes[i].updatedAt).toBeGreaterThanOrEqual(
			allNotes[i - 1].updatedAt,
		);
	}
});

test("Database notes handles empty content", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const notes = client.notes.getOrCreate(["test-empty"]);

	// Create note with empty content
	const emptyNote = await notes.updateNote({ content: "" });
	expect(emptyNote.content).toBe("");
	expect(emptyNote.id).toBeTruthy();
	expect(emptyNote.updatedAt).toBeGreaterThan(0);

	// Verify it was stored
	const allNotes = await notes.getNotes();
	expect(allNotes).toHaveLength(1);
	expect(allNotes[0]).toEqual(emptyNote);
});
