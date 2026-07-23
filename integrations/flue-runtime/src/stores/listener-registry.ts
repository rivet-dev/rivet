export class ListenerRegistry {
	private listeners = new Map<string, Set<() => void>>();

	subscribe(path: string, listener: () => void): () => void {
		let bucket = this.listeners.get(path);
		if (!bucket) {
			bucket = new Set();
			this.listeners.set(path, bucket);
		}
		bucket.add(listener);
		return () => {
			bucket.delete(listener);
			if (bucket.size === 0) this.listeners.delete(path);
		};
	}

	notify(path: string): void {
		for (const listener of this.listeners.get(path) ?? []) {
			try {
				listener();
			} catch {
				// Listener failures are intentionally isolated from writes.
			}
		}
	}
}
