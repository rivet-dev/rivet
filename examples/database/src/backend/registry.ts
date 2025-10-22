import { actor, setup } from "rivetkit";
import { authenticate } from "./my-utils";

export type Note = { id: string; content: string; updatedAt: number };

export const notes = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		notes: [] as Note[],
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		getNotes: (c) => c.state.notes,

		updateNote: (c, { id, content }: { id?: string; content: string }) => {
			const noteIndex = c.state.notes.findIndex((note) => note.id === id);
			let note: Note;

			if (noteIndex >= 0) {
				// Update existing note
				note = c.state.notes[noteIndex];
				note.content = content;
				note.updatedAt = Date.now();
				// Send events to all connected clients: https://rivet.dev/docs/actors/events
				c.broadcast("noteUpdated", note);
			} else {
				// Create new note
				note = {
					id: id || `note-${Date.now()}`,
					content,
					updatedAt: Date.now(),
				};
				// State changes are automatically persisted
				c.state.notes.push(note);
				c.broadcast("noteAdded", note);
			}

			return note;
		},

		deleteNote: (c, { id }: { id: string }) => {
			const noteIndex = c.state.notes.findIndex((note) => note.id === id);
			if (noteIndex >= 0) {
				c.state.notes.splice(noteIndex, 1);
				c.broadcast("noteDeleted", { id });
				return true;
			}
			return false;
		},
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { notes },
});
