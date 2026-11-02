import { createRivetKit } from "@rivetkit/react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { OperationEvent, registry } from "../src/index.ts";
import { type Operation, transformIndex } from "../src/ot.ts";
import { DocumentSync } from "./sync.ts";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

type View = { text: string; revision: number; pending: number };
type CaretRange = [start: number, end: number];

// Moves the local selection past text that another editor inserted or deleted.
function rebaseSelection(
	textarea: HTMLTextAreaElement | null,
	current: CaretRange | null,
	remote: Operation,
): CaretRange | null {
	if (!textarea || window.document.activeElement !== textarea) return null;
	const [start, end] = current ?? [
		textarea.selectionStart,
		textarea.selectionEnd,
	];
	return [transformIndex(remote, start), transformIndex(remote, end)];
}

function Editor({ documentId }: { documentId: string }) {
	const [view, setView] = useState<View | null>(null);
	const textareaRef = useRef<HTMLTextAreaElement | null>(null);
	const selectionRef = useRef<CaretRange | null>(null);
	const syncRef = useRef<DocumentSync | null>(null);

	// docs:start connect
	const documentActor = useActor({ name: "document", key: [documentId] });

	useEffect(() => {
		const connection = documentActor.connection;
		if (!connection) return;

		const clientId = crypto.randomUUID();
		const resync = () => {
			connection.getSnapshot().then((snapshot) => sync.load(snapshot));
		};
		const sync = new DocumentSync(
			clientId,
			(baseRevision, operation) =>
				connection.submitOperation(clientId, baseRevision, operation),
			(remote) => {
				if (remote) {
					selectionRef.current = rebaseSelection(
						textareaRef.current,
						selectionRef.current,
						remote,
					);
				}
				setView({
					text: sync.text,
					revision: sync.revision,
					pending: sync.pending.length,
				});
			},
			resync,
		);
		syncRef.current = sync;
		resync();

		return () => {
			syncRef.current = null;
		};
	}, [documentActor.connection]);

	documentActor.useEvent("operation", (event) => {
		syncRef.current?.receive(event as OperationEvent);
	});
	// docs:end connect

	// Restore the rebased selection after React writes the new text.
	useLayoutEffect(() => {
		const selection = selectionRef.current;
		if (selection && textareaRef.current) {
			textareaRef.current.setSelectionRange(selection[0], selection[1]);
			selectionRef.current = null;
		}
	});

	return (
		<section className="editor">
			<header className="editor-header">
				<h2>{documentId}</h2>
				<div className="editor-status">
					<span
						className={`status-dot ${view ? "online" : "offline"}`}
					/>
					{view
						? `Revision ${view.revision}, ${view.pending} pending`
						: "Connecting"}
				</div>
			</header>
			<textarea
				ref={textareaRef}
				value={view?.text ?? ""}
				disabled={!view}
				onChange={(event) => syncRef.current?.edit(event.target.value)}
				placeholder="Start typing. Open this page in a second tab to edit together."
			/>
		</section>
	);
}

export function App() {
	const [documentId, setDocumentId] = useState("welcome");

	return (
		<div className="page">
			<header className="hero">
				<div>
					<p className="eyebrow">Collaborative Document (OT)</p>
					<h1>Operational transformation with Rivet Actors</h1>
					<p>
						Each document is a Rivet Actor that orders every edit,
						transforms concurrent edits, and keeps an operation log
						in SQLite.
					</p>
				</div>
				<label className="hero-card">
					Document
					<input
						value={documentId}
						onChange={(event) => setDocumentId(event.target.value)}
						placeholder="document-id"
					/>
				</label>
			</header>
			<Editor key={documentId} documentId={documentId} />
		</div>
	);
}
