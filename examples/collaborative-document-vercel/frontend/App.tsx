import { createRivetKit } from "@rivetkit/react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import type {
	AwarenessEvent,
	DocumentSummary,
	SyncEvent,
	registry,
} from "../src/actors.ts";
import { Awareness, applyAwarenessUpdate, encodeAwarenessUpdate } from "y-protocols/awareness";
import * as Y from "yjs";

const { useActor } = createRivetKit<typeof registry>(`${location.origin}/api/rivet`);

const CURSOR_COLORS = [
	"#f97316",
	"#facc15",
	"#22c55e",
	"#06b6d4",
	"#3b82f6",
	"#8b5cf6",
	"#ec4899",
	"#f43f5e",
];

type PresenceEntry = {
	clientId: number;
	user?: { name: string; color: string };
	cursor?: { index: number; length: number } | null;
};

type RemoteCursor = {
	clientId: number;
	name: string;
	color: string;
	index: number;
};

const toNumbers = (update: Uint8Array) => Array.from(update);
const toBytes = (update: number[]) => Uint8Array.from(update);

const buildPresence = (awareness: Awareness): PresenceEntry[] => {
	return Array.from(awareness.getStates().entries()).map(([clientId, state]) => {
		return {
			clientId,
			user: state.user as PresenceEntry["user"],
			cursor: state.cursor as PresenceEntry["cursor"],
		};
	});
};

const renderWithCursors = (text: string, cursors: RemoteCursor[]) => {
	if (cursors.length === 0) {
		return text;
	}
	const ordered = [...cursors].sort((a, b) => {
		if (a.index !== b.index) return a.index - b.index;
		return a.clientId - b.clientId;
	});
	const parts: ReactNode[] = [];
	let lastIndex = 0;
	for (const cursor of ordered) {
		const clamped = Math.max(0, Math.min(cursor.index, text.length));
		if (clamped > lastIndex) {
			parts.push(text.slice(lastIndex, clamped));
		}
		parts.push(
			<span
				key={`cursor-${cursor.clientId}-${clamped}`}
				className="cursor-marker"
				data-name={cursor.name}
				style={{
					"--cursor-color": cursor.color,
				} as CSSProperties}
			/>,
		);
		lastIndex = clamped;
	}
	if (lastIndex < text.length) {
		parts.push(text.slice(lastIndex));
	}
	return parts;
};

type DocumentEditorProps = {
	workspaceId: string;
	document: DocumentSummary;
	username: string;
	userColor: string;
};

function DocumentEditor({
	workspaceId,
	document,
	username,
	userColor,
}: DocumentEditorProps) {
	const documentActor = useActor({
		name: "document",
		key: [workspaceId, document.id],
	});

	const [content, setContent] = useState("");
	const [presence, setPresence] = useState<PresenceEntry[]>([]);
	const textareaRef = useRef<HTMLTextAreaElement | null>(null);
	const connectionRef = useRef(documentActor.connection);
	const docRef = useRef<Y.Doc | null>(null);
	const textRef = useRef<Y.Text | null>(null);
	const awarenessRef = useRef<Awareness | null>(null);

	useEffect(() => {
		connectionRef.current = documentActor.connection;
	}, [documentActor.connection]);

	useEffect(() => {
		setContent("");
		setPresence([]);

		const doc = new Y.Doc();
		const text = doc.getText("content");
		const awareness = new Awareness(doc);
		docRef.current = doc;
		textRef.current = text;
		awarenessRef.current = awareness;

		const handleDocUpdate = (update: Uint8Array, origin: unknown) => {
			setContent(text.toString());
			if (origin === "remote") {
				return;
			}
			const connection = connectionRef.current;
			if (!connection) {
				return;
			}
			connection.applyUpdate(toNumbers(update), "sync");
		};

		const handleAwarenessUpdate = (
			{ added, updated, removed }: { added: number[]; updated: number[]; removed: number[] },
			origin: unknown,
		) => {
			setPresence(buildPresence(awareness));
			if (origin === "remote") {
				return;
			}
			const connection = connectionRef.current;
			if (!connection) {
				return;
			}
			const update = encodeAwarenessUpdate(awareness, [
				...added,
				...updated,
				...removed,
			]);
			connection.applyUpdate(toNumbers(update), "awareness", awareness.clientID);
		};

		doc.on("update", handleDocUpdate);
		awareness.on("update", handleAwarenessUpdate);

		return () => {
			doc.off("update", handleDocUpdate);
			awareness.off("update", handleAwarenessUpdate);
			awareness.destroy();
			doc.destroy();
		};
	}, [document.id]);

	useEffect(() => {
		const awareness = awarenessRef.current;
		if (!awareness) {
			return;
		}
		awareness.setLocalStateField("user", { name: username, color: userColor });
	}, [username, userColor, document.id, documentActor.connection]);

	useEffect(() => {
		const connection = documentActor.connection;
		const doc = docRef.current;
		const awareness = awarenessRef.current;
		const text = textRef.current;
		if (!connection || !doc || !awareness || !text) {
			return;
		}

		let cancelled = false;

		connection
			.getContent()
			.then((update) => {
				if (cancelled) {
					return;
				}
				Y.applyUpdate(doc, toBytes(update), "remote");
				setContent(text.toString());
			})
			.catch(() => null);

		connection
			.getAwareness()
			.then((update) => {
				if (cancelled) {
					return;
				}
				applyAwarenessUpdate(awareness, toBytes(update), "remote");
				setPresence(buildPresence(awareness));
			})
			.catch(() => null);

		return () => {
			cancelled = true;
		};
	}, [documentActor.connection, document.id]);

	documentActor.useEvent("sync", (event: SyncEvent) => {
		const doc = docRef.current;
		if (!doc) {
			return;
		}
		Y.applyUpdate(doc, toBytes(event.update), "remote");
	});

	documentActor.useEvent("awareness", (event: AwarenessEvent) => {
		const awareness = awarenessRef.current;
		if (!awareness) {
			return;
		}
		applyAwarenessUpdate(awareness, toBytes(event.update), "remote");
		setPresence(buildPresence(awareness));
	});

	const updateLocalCursor = () => {
		const awareness = awarenessRef.current;
		const textarea = textareaRef.current;
		if (!awareness || !textarea) {
			return;
		}
		const start = textarea.selectionStart ?? 0;
		const end = textarea.selectionEnd ?? start;
		awareness.setLocalStateField("cursor", {
			index: start,
			length: Math.max(0, end - start),
		});
	};

	const clearCursor = () => {
		const awareness = awarenessRef.current;
		if (!awareness) {
			return;
		}
		awareness.setLocalStateField("cursor", null);
	};

	const handleChange = (value: string) => {
		const doc = docRef.current;
		const text = textRef.current;
		if (!doc || !text) {
			return;
		}
		const current = text.toString();
		if (value === current) {
			return;
		}
		doc.transact(() => {
			text.delete(0, text.length);
			text.insert(0, value);
		}, "local");
	};

	const awareness = awarenessRef.current;
	const localClientId = awareness?.clientID;
	const remoteCursors: RemoteCursor[] = presence
		.filter((entry) => entry.clientId !== localClientId)
		.filter((entry) => entry.cursor && entry.user)
		.map((entry) => ({
			clientId: entry.clientId,
			name: entry.user?.name ?? "Anonymous",
			color: entry.user?.color ?? "#94a3b8",
			index: entry.cursor?.index ?? 0,
		}));
	const cursorOverlay = renderWithCursors(content, remoteCursors);

	return (
		<section className="editor">
			<header className="editor-header">
				<div>
					<h2>{document.title}</h2>
					<p>Document ID: {document.id}</p>
				</div>
				<div className="editor-status">
					<span
						className={`status-dot ${documentActor.connection ? "online" : "offline"}`}
					/>
					{documentActor.connection ? "Connected" : "Connecting"}
				</div>
			</header>

			<div className="editor-body">
				<div className="editor-panel">
					<div className="editor-canvas">
						<div className="cursor-overlay">{cursorOverlay}</div>
						<textarea
							ref={textareaRef}
							value={content}
							onChange={(event) => handleChange(event.target.value)}
							onSelect={updateLocalCursor}
							onKeyUp={updateLocalCursor}
							onMouseUp={updateLocalCursor}
							onBlur={clearCursor}
							placeholder="Start typing to collaborate"
						/>
					</div>
					<p className="editor-hint">
						This editor uses Yjs updates broadcast through the document actor.
					</p>
				</div>

				<div className="presence-panel">
					<h3>Active collaborators</h3>
					{presence.length === 0 ? (
						<p className="empty">No collaborators connected yet.</p>
					) : (
						<ul>
							{presence.map((entry) => (
								<li key={entry.clientId}>
									<span
										className="presence-color"
										style={{
											"--cursor-color": entry.user?.color ?? "#94a3b8",
										} as CSSProperties}
									/>
									<div>
										<strong>{entry.user?.name ?? "Anonymous"}</strong>
										<span>
											Cursor: {entry.cursor ? entry.cursor.index : "idle"}
										</span>
									</div>
								</li>
							))}
						</ul>
					)}
				</div>
			</div>
		</section>
	);
}

export function App() {
	const [workspaceId, setWorkspaceId] = useState("design-team");
	const [username, setUsername] = useState("Ada");
	const [newTitle, setNewTitle] = useState("Product spec");
	const [documents, setDocuments] = useState<DocumentSummary[]>([]);
	const [activeDocumentId, setActiveDocumentId] = useState<string | null>(null);

	const userColor = useMemo(() => {
		const hash = Array.from(username).reduce((sum, char) => sum + char.charCodeAt(0), 0);
		return CURSOR_COLORS[hash % CURSOR_COLORS.length];
	}, [username]);

	const documentList = useActor({
		name: "documentList",
		key: [workspaceId],
	});

	useEffect(() => {
		if (!documentList.connection) {
			return;
		}
		documentList.connection.listDocuments().then((list) => {
			setDocuments(list);
			setActiveDocumentId((current) => current ?? list[0]?.id ?? null);
		});
	}, [documentList.connection, workspaceId]);

	const handleCreateDocument = async () => {
		if (!documentList.connection) {
			return;
		}
		const doc = await documentList.connection.createDocument(newTitle);
		setDocuments((prev) => [...prev, doc]);
		setActiveDocumentId(doc.id);
		setNewTitle("Untitled document");
	};

	const handleDeleteDocument = async (docId: string) => {
		if (!documentList.connection) {
			return;
		}
		await documentList.connection.deleteDocument(docId);
		setDocuments((prev) => prev.filter((doc) => doc.id !== docId));
		if (activeDocumentId === docId) {
			setActiveDocumentId(null);
		}
	};

	const activeDocument = documents.find((doc) => doc.id === activeDocumentId) ?? null;

	return (
		<div className="page">
			<header className="hero">
				<div>
					<p className="eyebrow">Collaborative Document</p>
					<h1>Shared documents with Yjs and Rivet Actors</h1>
					<p>
						Document actors keep CRDT state in KV storage, while the coordinator
						indexes documents per workspace.
					</p>
				</div>
				<div className="hero-card">
					<label>
						Workspace
						<input
							value={workspaceId}
							onChange={(event) => setWorkspaceId(event.target.value)}
							placeholder="workspace-id"
						/>
					</label>
					<label>
						Name
						<input
							value={username}
							onChange={(event) => setUsername(event.target.value)}
							placeholder="Your name"
						/>
					</label>
				<div className="color-chip" style={{ "--cursor-color": userColor } as CSSProperties}>
						Active color
					</div>
				</div>
			</header>

			<section className="documents">
				<div className="documents-header">
					<h2>Workspace documents</h2>
					<div className="create-row">
						<input
							value={newTitle}
							onChange={(event) => setNewTitle(event.target.value)}
							placeholder="New document title"
						/>
						<button
							onClick={handleCreateDocument}
							disabled={!documentList.connection}
						>
							Create
						</button>
					</div>
				</div>
				<div className="documents-list">
					{documents.length === 0 ? (
						<p className="empty">No documents yet. Create one to start.</p>
					) : (
						documents.map((doc) => (
							<article
								key={doc.id}
								className={
									doc.id === activeDocumentId ? "document-card active" : "document-card"
								}
							>
								<div>
									<h3>{doc.title}</h3>
									<p>{new Date(doc.createdAt).toLocaleString()}</p>
								</div>
								<div className="document-actions">
									<button onClick={() => setActiveDocumentId(doc.id)}>
										Open
									</button>
									<button
										className="secondary"
										onClick={() => handleDeleteDocument(doc.id)}
									>
										Delete
									</button>
								</div>
							</article>
						))
					)}
				</div>
			</section>

			{activeDocument ? (
				<DocumentEditor
					workspaceId={workspaceId}
					document={activeDocument}
					username={username}
					userColor={userColor}
				/>
			) : (
				<section className="editor empty">
					<h2>Select a document</h2>
					<p>Choose a document from the workspace list to start editing.</p>
				</section>
			)}
		</div>
	);
}
