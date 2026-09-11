export type Status = "idle" | "connecting" | "connected" | "disconnected";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function createMockConnection(): {
  connection: any;
  setStatus(next: Status): void;
  emitError(message: string): void;
  emit(eventName: string, ...args: unknown[]): void;
} {
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
      statusListeners.forEach((listener) => listener(status));
    },
    ping() {
      return "pong";
    },
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  } as any;

  return {
    connection,
    setStatus(next: Status) {
      status = next;
      statusListeners.forEach((listener) => listener(status));
    },
    emitError(message: string) {
      const error = new Error(message);
      errorListeners.forEach((listener) => listener(error));
    },
    emit(eventName: string, ...args: unknown[]) {
      eventListeners.get(eventName)?.forEach((listener) => listener(...args));
    },
  };
}
