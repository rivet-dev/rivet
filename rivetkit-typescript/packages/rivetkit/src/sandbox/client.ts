/**
 * Client-side helpers for direct operations against a sandbox-agent server.
 *
 * The sandbox actor proxies all sandbox-agent SDK methods as Rivet Actor
 * actions, which use JSON-based RPC serialization. This works well for
 * structured data (sessions, processes, MCP config), but three categories of
 * operations do not fit through JSON actions:
 *
 * 1. Binary filesystem I/O (readFsFile, writeFsFile, uploadFsBatch): raw
 *    binary payloads would require base64 encoding with ~33% size overhead.
 * 2. WebSocket terminals (connectProcessTerminal,
 *    connectProcessTerminalWebSocket): bidirectional binary streams cannot be
 *    serialized through request-response JSON.
 * 3. SSE log streaming (followProcessLogs): continuous event streams with
 *    callbacks cannot be proxied through one-shot JSON actions.
 *
 * These helpers let the client talk directly to the sandbox-agent HTTP API,
 * bypassing the actor's JSON action layer. The sandbox URL is obtained via
 * the actor's `getSandboxUrl` action. Sandbox providers already secure the
 * connection between client and sandbox, so no additional authentication is
 * needed on these direct endpoints.
 */

const API_PREFIX = "/v1";

type FetchBody =
	| Blob
	| ArrayBuffer
	| Uint8Array
	| ReadableStream
	| string;

type TerminalInput = string | ArrayBuffer | ArrayBufferView;
type WebSocketConstructor = typeof WebSocket;
type ProcessLogStream = "stdout" | "stderr" | "combined" | "pty";

export interface FsEntry {
	entryType: "file" | "directory";
	modified?: string | null;
	name: string;
	path: string;
	size: number;
}

export interface FsStat {
	entryType: "file" | "directory";
	modified?: string | null;
	path: string;
	size: number;
}

export interface UploadBatchResponse {
	paths: string[];
	truncated: boolean;
}

export interface ProcessLogEntry {
	data: string;
	encoding: string;
	sequence: number;
	stream: ProcessLogStream;
	timestampMs: number;
}

export interface FollowProcessLogsOptions {
	stream?: ProcessLogStream;
	tail?: number;
	since?: number;
}

export interface TerminalExitStatus {
	exitCode: number | null;
}

export interface TerminalConnectOptions {
	protocols?: string | string[];
	WebSocket?: WebSocketConstructor;
}

export interface TerminalSession {
	onData(listener: (data: Uint8Array) => void): () => void;
	onExit(listener: (status: TerminalExitStatus) => void): () => void;
	onError(listener: (error: Error) => void): () => void;
	onClose(listener: () => void): () => void;
	sendInput(data: TerminalInput): void;
	resize(cols: number, rows: number): void;
	close(): void;
	socket: WebSocket;
}

export async function uploadFile(
	sandboxUrl: string,
	path: string,
	data: FetchBody,
): Promise<void> {
	const response = await fetchSandbox(
		buildUrl(sandboxUrl, `${API_PREFIX}/fs/file`, { path }),
		{
			method: "PUT",
			headers: {
				"Content-Type": "application/octet-stream",
			},
			body: data,
		},
	);
	await assertOk(response, "upload file");
}

export async function downloadFile(
	sandboxUrl: string,
	path: string,
): Promise<ArrayBuffer> {
	const response = await fetchSandbox(
		buildUrl(sandboxUrl, `${API_PREFIX}/fs/file`, { path }),
		{
			method: "GET",
		},
	);
	await assertOk(response, "download file");
	return await response.arrayBuffer();
}

export async function uploadBatch(
	sandboxUrl: string,
	destinationPath: string,
	tarData: Blob | ArrayBuffer | Uint8Array,
): Promise<UploadBatchResponse> {
	const response = await fetchSandbox(
		buildUrl(sandboxUrl, `${API_PREFIX}/fs/upload-batch`, {
			path: destinationPath,
		}),
		{
			method: "POST",
			headers: {
				"Content-Type": "application/x-tar",
			},
			body: tarData,
		},
	);
	await assertOk(response, "upload batch");
	return (await response.json()) as UploadBatchResponse;
}

export async function listFiles(
	sandboxUrl: string,
	path: string,
): Promise<FsEntry[]> {
	const response = await fetchSandbox(
		buildUrl(sandboxUrl, `${API_PREFIX}/fs/entries`, { path }),
		{
			method: "GET",
		},
	);
	await assertOk(response, "list files");
	return (await response.json()) as FsEntry[];
}

export async function statFile(
	sandboxUrl: string,
	path: string,
): Promise<FsStat> {
	const response = await fetchSandbox(
		buildUrl(sandboxUrl, `${API_PREFIX}/fs/stat`, { path }),
		{
			method: "GET",
		},
	);
	await assertOk(response, "stat file");
	return (await response.json()) as FsStat;
}

export async function deleteFile(
	sandboxUrl: string,
	path: string,
): Promise<void> {
	const response = await fetchSandbox(
		buildUrl(sandboxUrl, `${API_PREFIX}/fs/entry`, { path }),
		{
			method: "DELETE",
		},
	);
	await assertOk(response, "delete file");
}

export async function mkdirFs(
	sandboxUrl: string,
	path: string,
): Promise<void> {
	const response = await fetchSandbox(
		buildUrl(sandboxUrl, `${API_PREFIX}/fs/mkdir`, { path }),
		{
			method: "POST",
		},
	);
	await assertOk(response, "mkdir");
}

export async function moveFile(
	sandboxUrl: string,
	from: string,
	to: string,
	overwrite = false,
): Promise<void> {
	const response = await fetchSandbox(
		buildUrl(sandboxUrl, `${API_PREFIX}/fs/move`),
		{
			method: "POST",
			headers: {
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				from,
				to,
				overwrite,
			}),
		},
	);
	await assertOk(response, "move file");
}

export function buildTerminalWebSocketUrl(
	sandboxUrl: string,
	processId: string,
): string {
	const url = new URL(
		buildUrl(
			sandboxUrl,
			`${API_PREFIX}/processes/${encodeURIComponent(processId)}/terminal/ws`,
		),
	);
	if (url.protocol === "http:") {
		url.protocol = "ws:";
	} else if (url.protocol === "https:") {
		url.protocol = "wss:";
	}
	return url.toString();
}

export function connectTerminal(
	sandboxUrl: string,
	processId: string,
	options: TerminalConnectOptions = {},
): TerminalSession {
	const WebSocketCtor = options.WebSocket ?? getWebSocketCtor();
	const socket = new WebSocketCtor(
		buildTerminalWebSocketUrl(sandboxUrl, processId),
		options.protocols,
	);
	socket.binaryType = "arraybuffer";
	return new DirectTerminalSession(socket);
}

export async function followProcessLogs(
	sandboxUrl: string,
	processId: string,
	listener: (entry: ProcessLogEntry) => void,
	options: FollowProcessLogsOptions = {},
): Promise<{ close: () => void; closed: Promise<void> }> {
	const abortController = new AbortController();
	const response = await fetchSandbox(
		buildUrl(
			sandboxUrl,
			`${API_PREFIX}/processes/${encodeURIComponent(processId)}/logs`,
			{
				follow: true,
				stream: options.stream,
				tail: options.tail,
				since: options.since,
			},
		),
		{
			method: "GET",
			headers: {
				Accept: "text/event-stream",
			},
			signal: abortController.signal,
		},
	);
	await assertOk(response, "follow process logs");
	if (!response.body) {
		abortController.abort();
		throw new Error("SSE stream is not readable in this environment.");
	}

	const closed = consumeProcessLogSse(
		response.body,
		listener,
		abortController.signal,
	);
	return {
		close: () => abortController.abort(),
		closed,
	};
}

async function assertOk(response: Response, operation: string): Promise<void> {
	if (!response.ok) {
		const body = await response.text().catch(() => "");
		throw new Error(
			`Sandbox ${operation} failed (${response.status}): ${body}`,
		);
	}
}

class DirectTerminalSession implements TerminalSession {
	readonly socket: WebSocket;

	private readonly dataListeners = new Set<(data: Uint8Array) => void>();
	private readonly exitListeners = new Set<
		(status: TerminalExitStatus) => void
	>();
	private readonly errorListeners = new Set<(error: Error) => void>();
	private readonly closeListeners = new Set<() => void>();
	private closeSignalSent = false;

	constructor(socket: WebSocket) {
		this.socket = socket;
		this.socket.addEventListener("message", (event) => {
			void this.handleMessage(event.data);
		});
		this.socket.addEventListener("error", () => {
			this.emitError(new Error("Terminal websocket connection failed."));
		});
		this.socket.addEventListener("close", () => {
			for (const listener of this.closeListeners) {
				listener();
			}
		});
	}

	onData(listener: (data: Uint8Array) => void): () => void {
		this.dataListeners.add(listener);
		return () => this.dataListeners.delete(listener);
	}

	onExit(listener: (status: TerminalExitStatus) => void): () => void {
		this.exitListeners.add(listener);
		return () => this.exitListeners.delete(listener);
	}

	onError(listener: (error: Error) => void): () => void {
		this.errorListeners.add(listener);
		return () => this.errorListeners.delete(listener);
	}

	onClose(listener: () => void): () => void {
		this.closeListeners.add(listener);
		return () => this.closeListeners.delete(listener);
	}

	sendInput(data: TerminalInput): void {
		const payload = encodeTerminalInput(data);
		this.sendFrame({
			type: "input",
			data: payload.data,
			encoding: payload.encoding,
		});
	}

	resize(cols: number, rows: number): void {
		this.sendFrame({
			type: "resize",
			cols,
			rows,
		});
	}

	close(): void {
		if (this.socket.readyState === 0) {
			this.socket.addEventListener(
				"open",
				() => {
					this.close();
				},
				{ once: true },
			);
			return;
		}

		if (this.socket.readyState === 1) {
			if (!this.closeSignalSent) {
				this.closeSignalSent = true;
				this.sendFrame({ type: "close" });
			}
			this.socket.close(1000, "sandbox.client_closed");
			return;
		}

		if (this.socket.readyState !== 3) {
			this.socket.close(1000, "sandbox.client_closed");
		}
	}

	private async handleMessage(data: unknown): Promise<void> {
		try {
			if (typeof data === "string") {
				const frame = parseTerminalServerFrame(data);
				if (!frame) {
					this.emitError(
						new Error("Received invalid terminal control frame."),
					);
					return;
				}

				if (frame.type === "exit") {
					for (const listener of this.exitListeners) {
						listener({ exitCode: frame.exitCode ?? null });
					}
					return;
				}

				if (frame.type === "error") {
					this.emitError(new Error(frame.message));
				}
				return;
			}

			const bytes = await decodeTerminalBytes(data);
			if (!bytes) {
				this.emitError(
					new Error("Received unsupported terminal message payload."),
				);
				return;
			}

			for (const listener of this.dataListeners) {
				listener(bytes);
			}
		} catch (error) {
			this.emitError(
				error instanceof Error ? error : new Error(String(error)),
			);
		}
	}

	private sendFrame(frame: {
		type: "input";
		data: string;
		encoding?: string;
	} | {
		type: "resize";
		cols: number;
		rows: number;
	} | {
		type: "close";
	}): void {
		if (this.socket.readyState !== 1) {
			return;
		}
		this.socket.send(JSON.stringify(frame));
	}

	private emitError(error: Error): void {
		for (const listener of this.errorListeners) {
			listener(error);
		}
	}
}

async function fetchSandbox(
	input: string,
	init: RequestInit & { body?: FetchBody },
): Promise<Response> {
	const requestInit = { ...init } as RequestInit & {
		body?: unknown;
		duplex?: "half";
	};
	requestInit.body = init.body;
	if (isReadableStream(init.body)) {
		requestInit.duplex = "half";
	}
	return await getFetch()(input, requestInit);
}

function getFetch(): typeof fetch {
	if (!globalThis.fetch) {
		throw new Error(
			"Fetch API is not available; provide a global fetch implementation.",
		);
	}
	return globalThis.fetch.bind(globalThis);
}

function getWebSocketCtor(): WebSocketConstructor {
	if (!globalThis.WebSocket) {
		throw new Error(
			"WebSocket API is not available; provide a WebSocket implementation.",
		);
	}
	return globalThis.WebSocket;
}

function buildUrl(
	sandboxUrl: string,
	pathname: string,
	query?: Record<string, string | number | boolean | undefined>,
): string {
	const url = new URL(sandboxUrl);
	const basePath = url.pathname.replace(/\/+$/, "");
	const suffix = pathname.startsWith("/") ? pathname : `/${pathname}`;
	url.pathname = `${basePath}${suffix}`.replace(/\/{2,}/g, "/");
	if (query) {
		for (const [key, value] of Object.entries(query)) {
			if (value === undefined) {
				continue;
			}
			url.searchParams.set(key, String(value));
		}
	}
	return url.toString();
}

function parseTerminalServerFrame(payload: string):
	| {
			type: "ready";
			processId: string;
	  }
	| {
			type: "exit";
			exitCode?: number | null;
	  }
	| {
			type: "error";
			message: string;
	  }
	| null {
	try {
		const parsed = JSON.parse(payload) as Record<string, unknown>;
		if (typeof parsed.type !== "string") {
			return null;
		}
		if (
			parsed.type === "ready" &&
			typeof parsed.processId === "string"
		) {
			return {
				type: "ready",
				processId: parsed.processId,
			};
		}
		if (
			parsed.type === "exit" &&
			(parsed.exitCode === undefined ||
				parsed.exitCode === null ||
				typeof parsed.exitCode === "number")
		) {
			return {
				type: "exit",
				exitCode: (parsed.exitCode as number | null | undefined) ?? null,
			};
		}
		if (
			parsed.type === "error" &&
			typeof parsed.message === "string"
		) {
			return {
				type: "error",
				message: parsed.message,
			};
		}
	} catch {
		return null;
	}
	return null;
}

function encodeTerminalInput(data: TerminalInput): {
	data: string;
	encoding?: string;
} {
	if (typeof data === "string") {
		return { data };
	}
	return {
		data: bytesToBase64(encodeTerminalBytes(data)),
		encoding: "base64",
	};
}

function encodeTerminalBytes(data: ArrayBuffer | ArrayBufferView): Uint8Array {
	if (data instanceof ArrayBuffer) {
		return new Uint8Array(data);
	}
	return new Uint8Array(data.buffer, data.byteOffset, data.byteLength).slice();
}

async function decodeTerminalBytes(data: unknown): Promise<Uint8Array | null> {
	if (data instanceof ArrayBuffer) {
		return new Uint8Array(data);
	}
	if (ArrayBuffer.isView(data)) {
		return new Uint8Array(
			data.buffer,
			data.byteOffset,
			data.byteLength,
		).slice();
	}
	if (typeof Blob !== "undefined" && data instanceof Blob) {
		return new Uint8Array(await data.arrayBuffer());
	}
	return null;
}

function bytesToBase64(bytes: Uint8Array): string {
	const bufferCtor = (
		globalThis as typeof globalThis & {
			Buffer?: {
				from(data: Uint8Array): { toString(encoding: "base64"): string };
			};
		}
	).Buffer;
	if (bufferCtor) {
		return bufferCtor.from(bytes).toString("base64");
	}

	let binary = "";
	const chunkSize = 0x8000;
	for (let index = 0; index < bytes.length; index += chunkSize) {
		const chunk = bytes.subarray(index, index + chunkSize);
		binary += String.fromCharCode(...chunk);
	}
	if (typeof btoa !== "function") {
		throw new Error("No base64 encoder is available in this environment.");
	}
	return btoa(binary);
}

async function consumeProcessLogSse(
	body: ReadableStream<Uint8Array>,
	listener: (entry: ProcessLogEntry) => void,
	signal: AbortSignal,
): Promise<void> {
	const reader = body.getReader();
	const decoder = new TextDecoder();
	let buffer = "";
	try {
		while (!signal.aborted) {
			const { done, value } = await reader.read();
			if (done) {
				return;
			}
			buffer += decoder
				.decode(value, { stream: true })
				.replace(/\r\n/g, "\n");
			let separatorIndex = buffer.indexOf("\n\n");
			while (separatorIndex !== -1) {
				const chunk = buffer.slice(0, separatorIndex);
				buffer = buffer.slice(separatorIndex + 2);
				const entry = parseProcessLogSseChunk(chunk);
				if (entry) {
					listener(entry);
				}
				separatorIndex = buffer.indexOf("\n\n");
			}
		}
	} catch (error) {
		if (signal.aborted || isAbortError(error)) {
			return;
		}
		throw error;
	} finally {
		reader.releaseLock();
	}
}

function parseProcessLogSseChunk(chunk: string): ProcessLogEntry | null {
	if (!chunk.trim()) {
		return null;
	}

	let eventName = "message";
	const dataLines: string[] = [];
	for (const line of chunk.split("\n")) {
		if (!line || line.startsWith(":")) {
			continue;
		}
		if (line.startsWith("event:")) {
			eventName = line.slice(6).trim();
			continue;
		}
		if (line.startsWith("data:")) {
			dataLines.push(line.slice(5).trimStart());
		}
	}

	if (eventName !== "log") {
		return null;
	}

	const data = dataLines.join("\n");
	if (!data.trim()) {
		return null;
	}

	return JSON.parse(data) as ProcessLogEntry;
}

function isAbortError(error: unknown): boolean {
	return error instanceof Error && error.name === "AbortError";
}

function isReadableStream(value: unknown): value is ReadableStream {
	return (
		typeof ReadableStream !== "undefined" &&
		value instanceof ReadableStream
	);
}
