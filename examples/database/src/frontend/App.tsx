import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { Note, registry } from "../backend/registry";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

function NotesApp({ userId }: { userId: string }) {
	const [notes, setNotes] = useState<Note[]>([]);
	const [newNote, setNewNote] = useState("");
	const [editingNote, setEditingNote] = useState<string | null>(null);
	const [editContent, setEditContent] = useState("");

	const notesActor = useActor({
		name: "notes",
		key: [userId],
		params: { userId, token: "demo-token" },
	});

	useEffect(() => {
		if (notesActor.connection) {
			notesActor.connection.getNotes().then(setNotes);
		}
	}, [notesActor.connection]);

	notesActor.useEvent("noteAdded", (note: Note) => {
		setNotes((prev) => [...prev, note]);
	});

	notesActor.useEvent("noteUpdated", (updatedNote: Note) => {
		setNotes((prev) =>
			prev.map((note) => (note.id === updatedNote.id ? updatedNote : note))
		);
		setEditingNote(null);
	});

	notesActor.useEvent("noteDeleted", ({ id }: { id: string }) => {
		setNotes((prev) => prev.filter((note) => note.id !== id));
	});

	const addNote = async () => {
		if (notesActor.connection && newNote.trim()) {
			await notesActor.connection.updateNote({ 
				id: `note-${Date.now()}`, 
				content: newNote 
			});
			setNewNote("");
		}
	};

	const startEdit = (note: Note) => {
		setEditingNote(note.id);
		setEditContent(note.content);
	};

	const saveEdit = async () => {
		if (notesActor.connection && editingNote) {
			await notesActor.connection.updateNote({ 
				id: editingNote, 
				content: editContent 
			});
		}
	};

	const cancelEdit = () => {
		setEditingNote(null);
		setEditContent("");
	};

	const deleteNote = async (id: string) => {
		if (notesActor.connection && confirm("Are you sure you want to delete this note?")) {
			await notesActor.connection.deleteNote({ id });
		}
	};

	const handleKeyPress = (e: React.KeyboardEvent, action: () => void) => {
		if (e.key === "Enter") {
			action();
		}
	};

	return (
		<div className="notes-section">
			<div className={`connection-status ${notesActor.connection ? 'connected' : 'disconnected'}`}>
				{notesActor.connection ? '✓ Connected' : '⚠ Disconnected'}
			</div>

			<div className="add-note">
				<input
					type="text"
					value={newNote}
					onChange={(e) => setNewNote(e.target.value)}
					onKeyPress={(e) => handleKeyPress(e, addNote)}
					placeholder="Enter a new note..."
					disabled={!notesActor.connection}
				/>
				<button
					onClick={addNote}
					disabled={!notesActor.connection || !newNote.trim()}
				>
					Add Note
				</button>
			</div>

			{notes.length === 0 ? (
				<div className="empty-state">
					No notes yet. Add your first note above!
				</div>
			) : (
				<ul className="notes-list">
					{notes
						.sort((a, b) => b.updatedAt - a.updatedAt)
						.map((note) => (
						<li 
							key={note.id} 
							className={`note-item ${editingNote === note.id ? 'edit-mode' : ''}`}
						>
							{editingNote === note.id ? (
								<div style={{ width: "100%" }}>
									<input
										type="text"
										value={editContent}
										onChange={(e) => setEditContent(e.target.value)}
										onKeyPress={(e) => handleKeyPress(e, saveEdit)}
										className="edit-input"
										autoFocus
									/>
									<div className="edit-actions">
										<button onClick={saveEdit} className="save-btn">
											Save
										</button>
										<button onClick={cancelEdit} className="cancel-btn">
											Cancel
										</button>
									</div>
								</div>
							) : (
								<>
									<div className="note-content">
										<div>{note.content}</div>
										<div className="note-meta">
											Last updated: {new Date(note.updatedAt).toLocaleString()}
										</div>
									</div>
									<div className="note-actions">
										<button
											onClick={() => startEdit(note)}
											className="edit-btn"
										>
											Edit
										</button>
										<button
											onClick={() => deleteNote(note.id)}
											className="delete-btn"
										>
											Delete
										</button>
									</div>
								</>
							)}
						</li>
					))}
				</ul>
			)}
		</div>
	);
}

export function App() {
	const [selectedUser, setSelectedUser] = useState("user1");

	const users = [
		{ id: "user1", name: "Alice" },
		{ id: "user2", name: "Bob" },
		{ id: "user3", name: "Charlie" },
	];

	return (
		<div className="app-container">
			<div className="header">
				<h1>Database Notes</h1>
				<p>Persistent note-taking with real-time updates</p>
			</div>

			<div className="user-selector">
				<label>Select User:</label>
				<select
					value={selectedUser}
					onChange={(e) => setSelectedUser(e.target.value)}
				>
					{users.map((user) => (
						<option key={user.id} value={user.id}>
							{user.name} ({user.id})
						</option>
					))}
				</select>
			</div>

			<NotesApp key={selectedUser} userId={selectedUser} />
		</div>
	);
}
