import type { ManagedProcess } from "@secure-exec/core";
import {
	deserializeMessage,
	isResponse,
	type JsonRpcNotification,
	type JsonRpcResponse,
	serializeMessage,
} from "./protocol.js";

const DEFAULT_TIMEOUT_MS = 120_000;

interface PendingRequest {
	resolve: (response: JsonRpcResponse) => void;
	reject: (error: Error) => void;
	timer: ReturnType<typeof setTimeout>;
}

export type NotificationHandler = (notification: JsonRpcNotification) => void;

export class AcpClient {
	private _process: ManagedProcess;
	private _nextId = 1;
	private _pending = new Map<number, PendingRequest>();
	private _notificationHandlers: NotificationHandler[] = [];
	private _closed = false;
	private _timeoutMs: number;

	constructor(
		process: ManagedProcess,
		stdoutLines: AsyncIterable<string>,
		options?: { timeoutMs?: number },
	) {
		this._process = process;
		this._timeoutMs = options?.timeoutMs ?? DEFAULT_TIMEOUT_MS;
		this._startReading(stdoutLines);
		this._watchExit();
	}

	request(method: string, params?: unknown): Promise<JsonRpcResponse> {
		if (this._closed) {
			return Promise.reject(new Error("AcpClient is closed"));
		}

		const id = this._nextId++;
		const msg = serializeMessage({ jsonrpc: "2.0", id, method, params });
		this._process.writeStdin(msg);

		return new Promise<JsonRpcResponse>((resolve, reject) => {
			const timer = setTimeout(() => {
				this._pending.delete(id);
				reject(
					new Error(
						`ACP request ${method} (id=${id}) timed out after ${this._timeoutMs}ms`,
					),
				);
			}, this._timeoutMs);

			this._pending.set(id, { resolve, reject, timer });
		});
	}

	notify(method: string, params?: unknown): void {
		if (this._closed) return;
		const msg = serializeMessage({ jsonrpc: "2.0", method, params });
		this._process.writeStdin(msg);
	}

	onNotification(handler: NotificationHandler): void {
		this._notificationHandlers.push(handler);
	}

	close(): void {
		if (this._closed) return;
		this._closed = true;
		this._rejectAll(new Error("AcpClient closed"));
		this._process.kill();
	}

	private _startReading(stdoutLines: AsyncIterable<string>): void {
		void (async () => {
			try {
				for await (const line of stdoutLines) {
					if (this._closed) break;
					const trimmed = line.trim();
					if (!trimmed) continue;

					const msg = deserializeMessage(trimmed);
					if (!msg) continue; // Skip non-JSON lines

					if (isResponse(msg)) {
						const pending = this._pending.get(msg.id);
						if (pending) {
							this._pending.delete(msg.id);
							clearTimeout(pending.timer);
							pending.resolve(msg);
						}
					} else {
						for (const handler of this._notificationHandlers) {
							handler(msg);
						}
					}
				}
			} catch {
				// Stream ended or errored
			}
		})();
	}

	private _watchExit(): void {
		this._process.wait().then(() => {
			this._closed = true;
			this._rejectAll(new Error("Agent process exited"));
		});
	}

	private _rejectAll(error: Error): void {
		for (const [id, pending] of this._pending) {
			clearTimeout(pending.timer);
			pending.reject(error);
			this._pending.delete(id);
		}
	}
}
