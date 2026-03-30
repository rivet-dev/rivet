import type { ActorConn, AnyActorDefinition } from "rivetkit/client";

export type Status = "idle" | "connecting" | "connected" | "disconnected";

export function createMockConnection() {
	let status: Status = "idle";
	const statusListeners = new Set<(status: Status) => void>();
	const errorListeners = new Set<(error: Error) => void>();
	const eventListeners = new Map<string, Set<(...args: unknown[]) => void>>();

	const connection = {
		get connStatus() {
			return status;
		},
		onStatusChange(callback: (next: Status) => void) {
			statusListeners.add(callback);
			return () => statusListeners.delete(callback);
		},
		onError(callback: (error: Error) => void) {
			errorListeners.add(callback);
			return () => errorListeners.delete(callback);
		},
		on(eventName: string, callback: (...args: unknown[]) => void) {
			let listeners = eventListeners.get(eventName);
			if (!listeners) {
				listeners = new Set();
				eventListeners.set(eventName, listeners);
			}
			listeners.add(callback);
			return () => listeners?.delete(callback);
		},
		async dispose() {
			status = "disconnected";
			for (const listener of statusListeners) {
				listener(status);
			}
		},
		ping() {
			return "pong";
		},
	} as unknown as ActorConn<AnyActorDefinition> & { ping(): string };

	return {
		connection,
		setStatus(next: Status) {
			status = next;
			for (const listener of statusListeners) {
				listener(status);
			}
		},
		emitError(message: string) {
			const error = new Error(message);
			for (const listener of errorListeners) {
				listener(error);
			}
		},
		emit(eventName: string, ...args: unknown[]) {
			for (const listener of eventListeners.get(eventName) ?? []) {
				listener(...args);
			}
		},
	};
}
