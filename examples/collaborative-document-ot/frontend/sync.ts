import type { OperationEvent, Snapshot } from "../src/index.ts";
import { apply, diff, isNoop, type Operation, transform } from "../src/ot.ts";

type Send = (baseRevision: number, operation: Operation) => Promise<unknown>;

/**
 * Keeps one editor's copy of a document in sync with the `document` Actor.
 *
 * Local edits apply right away and wait in `pending`. Only the first pending
 * operation is sent, and the next one goes out when the Actor broadcasts it
 * back. Operations from other editors are transformed past every pending
 * operation before they are applied, so local edits never get lost.
 */
export class DocumentSync {
	text = "";
	revision = 0;
	pending: Operation[] = [];
	private loaded = false;
	private inFlight = false;
	private buffered = new Map<number, OperationEvent>();

	constructor(
		private readonly clientId: string,
		private readonly send: Send,
		private readonly onChange: (remote?: Operation) => void,
		private readonly onRejected: () => void,
	) {}

	// docs:start load
	/** Starts from a snapshot, then applies any events that arrived first. */
	load(snapshot: Snapshot) {
		this.text = snapshot.text;
		this.revision = snapshot.revision;
		this.pending = [];
		this.inFlight = false;
		this.loaded = true;
		this.drain();
		this.onChange();
	}

	/** Queues an event and applies events strictly in revision order. */
	receive(event: OperationEvent) {
		this.buffered.set(event.revision, event);
		if (this.loaded) this.drain();
	}

	private drain() {
		for (const revision of this.buffered.keys()) {
			if (revision <= this.revision) this.buffered.delete(revision);
		}
		let next = this.buffered.get(this.revision + 1);
		while (next) {
			this.buffered.delete(next.revision);
			this.applyEvent(next);
			next = this.buffered.get(this.revision + 1);
		}
	}
	// docs:end load

	// docs:start edit
	/** Records a local edit and sends it if nothing is in flight. */
	edit(newText: string) {
		const op = diff(this.text, newText);
		if (isNoop(op)) return;
		this.text = newText;
		this.pending.push(op);
		this.flush();
		this.onChange();
	}

	private flush() {
		if (this.inFlight || !this.loaded || this.pending.length === 0) return;
		this.inFlight = true;
		this.send(this.revision, this.pending[0]).catch((error) => {
			// The local text no longer matches the Actor, so start over from a
			// fresh snapshot.
			console.error("operation rejected", error);
			this.loaded = false;
			this.onRejected();
		});
	}
	// docs:end edit

	// docs:start rebase
	private applyEvent(event: OperationEvent) {
		this.revision = event.revision;

		// Our own operation came back, so the Actor accepted it.
		if (this.inFlight && event.clientId === this.clientId) {
			this.pending.shift();
			this.inFlight = false;
			this.flush();
			this.onChange();
			return;
		}

		// Another editor's operation: transform it past every local operation
		// the Actor has not committed yet, and rebase those on top of it.
		let remote = event.operation;
		this.pending = this.pending.map((local) => {
			const [rebased, remoteAfter] = transform(local, remote);
			remote = remoteAfter;
			return rebased;
		});
		this.text = apply(this.text, remote);
		this.onChange(remote);
	}
	// docs:end rebase
}
