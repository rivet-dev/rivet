import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import {
	buildTerminalWebSocketUrl,
	connectTerminal,
	deleteFile,
	downloadFile,
	followProcessLogs,
	listFiles,
	mkdirFs,
	moveFile,
	statFile,
	uploadBatch,
	uploadFile,
} from "./client";

class MockWebSocket {
	static instances: MockWebSocket[] = [];

	readonly url: string;
	readonly protocols?: string | string[];
	readonly sent: unknown[] = [];
	binaryType = "blob";
	readyState = 1;

	private readonly listeners = new Map<
		string,
		Array<{ listener: (event?: any) => void; once: boolean }>
	>();

	constructor(url: string, protocols?: string | string[]) {
		this.url = url;
		this.protocols = protocols;
		MockWebSocket.instances.push(this);
	}

	addEventListener(
		type: string,
		listener: (event?: any) => void,
		options?: EventListenerOptions & { once?: boolean },
	): void {
		const entries = this.listeners.get(type) ?? [];
		entries.push({
			listener,
			once: options?.once ?? false,
		});
		this.listeners.set(type, entries);
	}

	send(data: unknown): void {
		this.sent.push(data);
	}

	close(): void {
		this.readyState = 3;
		this.emit("close");
	}

	emit(type: string, event?: any): void {
		const entries = [...(this.listeners.get(type) ?? [])];
		for (const entry of entries) {
			entry.listener(event);
			if (entry.once) {
				const remaining = (this.listeners.get(type) ?? []).filter(
					(candidate) => candidate !== entry,
				);
				this.listeners.set(type, remaining);
			}
		}
	}
}

const originalFetch = globalThis.fetch;
const originalWebSocket = globalThis.WebSocket;

function setGlobalFetch(fetchImpl: typeof fetch): void {
	Object.defineProperty(globalThis, "fetch", {
		configurable: true,
		writable: true,
		value: fetchImpl,
	});
}

function restoreGlobals(): void {
	if (originalFetch) {
		Object.defineProperty(globalThis, "fetch", {
			configurable: true,
			writable: true,
			value: originalFetch,
		});
	} else {
		delete (globalThis as { fetch?: typeof fetch }).fetch;
	}

	if (originalWebSocket) {
		Object.defineProperty(globalThis, "WebSocket", {
			configurable: true,
			writable: true,
			value: originalWebSocket,
		});
	} else {
		delete (globalThis as { WebSocket?: typeof WebSocket }).WebSocket;
	}
}

async function withSandboxServer(
	handler: (
		req: IncomingMessage,
		res: ServerResponse,
		body: string,
	) => void | Promise<void>,
	run: (
		baseUrl: string,
		requests: Array<{ method: string; url: string; body: string }>,
	) => Promise<void>,
): Promise<void> {
	const requests: Array<{ method: string; url: string; body: string }> = [];
	const server = createServer(async (req, res) => {
		const chunks: Uint8Array[] = [];
		for await (const chunk of req) {
			chunks.push(
				typeof chunk === "string" ? new TextEncoder().encode(chunk) : chunk,
			);
		}
		const body = Buffer.concat(chunks).toString();
		requests.push({
			method: req.method ?? "GET",
			url: req.url ?? "/",
			body,
		});
		await handler(req, res, body);
	});

	await new Promise<void>((resolve) => {
		server.listen(0, "127.0.0.1", () => resolve());
	});

	const port = (server.address() as AddressInfo).port;
	try {
		await run(`http://127.0.0.1:${port}/base`, requests);
	} finally {
		await new Promise<void>((resolve, reject) => {
			server.close((error) => {
				if (error) {
					reject(error);
				} else {
					resolve();
				}
			});
		});
	}
}

describe("sandbox direct client helpers", () => {
	beforeEach(() => {
		MockWebSocket.instances = [];
	});

	afterEach(() => {
		vi.restoreAllMocks();
		restoreGlobals();
	});

	test("uploadFile and downloadFile use the raw file endpoint", async () => {
		await withSandboxServer(
			(req, res) => {
				if (
					req.method === "PUT" &&
					req.url === "/base/v1/fs/file?path=%2Fworkspace%2Fa.txt"
				) {
					res.writeHead(204);
					res.end();
					return;
				}
				if (
					req.method === "GET" &&
					req.url === "/base/v1/fs/file?path=%2Fworkspace%2Fa.txt"
				) {
					res.writeHead(200);
					res.end("hi");
					return;
				}
				res.writeHead(404);
				res.end();
			},
			async (baseUrl, requests) => {
				await uploadFile(baseUrl, "/workspace/a.txt", "hello");
				const downloaded = await downloadFile(baseUrl, "/workspace/a.txt");

				expect(new TextDecoder().decode(downloaded)).toBe("hi");
				expect(requests).toEqual([
					{
						method: "PUT",
						url: "/base/v1/fs/file?path=%2Fworkspace%2Fa.txt",
						body: "hello",
					},
					{
						method: "GET",
						url: "/base/v1/fs/file?path=%2Fworkspace%2Fa.txt",
						body: "",
					},
				]);
			},
		);
	});

	test("uploadBatch, listFiles, and statFile parse JSON responses", async () => {
		await withSandboxServer(
			(req, res) => {
				if (
					req.method === "POST" &&
					req.url === "/base/v1/fs/upload-batch?path=%2Fworkspace"
				) {
					res.writeHead(200, { "Content-Type": "application/json" });
					res.end(
						JSON.stringify({
							paths: ["/workspace/a.txt"],
							truncated: false,
						}),
					);
					return;
				}
				if (
					req.method === "GET" &&
					req.url === "/base/v1/fs/entries?path=%2Fworkspace"
				) {
					res.writeHead(200, { "Content-Type": "application/json" });
					res.end(
						JSON.stringify([
							{
								entryType: "file",
								name: "a.txt",
								path: "/workspace/a.txt",
								size: 2,
							},
						]),
					);
					return;
				}
				if (
					req.method === "GET" &&
					req.url === "/base/v1/fs/stat?path=%2Fworkspace%2Fa.txt"
				) {
					res.writeHead(200, { "Content-Type": "application/json" });
					res.end(
						JSON.stringify({
							entryType: "file",
							path: "/workspace/a.txt",
							size: 2,
						}),
					);
					return;
				}
				res.writeHead(404);
				res.end();
			},
			async (baseUrl, requests) => {
				await expect(
					uploadBatch(baseUrl, "/workspace", new Uint8Array([1, 2, 3])),
				).resolves.toEqual({
					paths: ["/workspace/a.txt"],
					truncated: false,
				});
				await expect(listFiles(baseUrl, "/workspace")).resolves.toEqual([
					{
						entryType: "file",
						name: "a.txt",
						path: "/workspace/a.txt",
						size: 2,
					},
				]);
				await expect(
					statFile(baseUrl, "/workspace/a.txt"),
				).resolves.toEqual({
					entryType: "file",
					path: "/workspace/a.txt",
					size: 2,
				});

				expect(requests.map((request) => request.url)).toEqual([
					"/base/v1/fs/upload-batch?path=%2Fworkspace",
					"/base/v1/fs/entries?path=%2Fworkspace",
					"/base/v1/fs/stat?path=%2Fworkspace%2Fa.txt",
				]);
			},
		);
	});

	test("deleteFile, mkdirFs, and moveFile use the expected HTTP methods", async () => {
		await withSandboxServer(
			(_req, res) => {
				res.writeHead(204);
				res.end();
			},
			async (baseUrl, requests) => {
				await deleteFile(baseUrl, "/workspace/a.txt");
				await mkdirFs(baseUrl, "/workspace/output");
				await moveFile(
					baseUrl,
					"/workspace/a.txt",
					"/workspace/output/a.txt",
					true,
				);

				expect(requests).toEqual([
					{
						method: "DELETE",
						url: "/base/v1/fs/entry?path=%2Fworkspace%2Fa.txt",
						body: "",
					},
					{
						method: "POST",
						url: "/base/v1/fs/mkdir?path=%2Fworkspace%2Foutput",
						body: "",
					},
					{
						method: "POST",
						url: "/base/v1/fs/move",
						body: JSON.stringify({
							from: "/workspace/a.txt",
							to: "/workspace/output/a.txt",
							overwrite: true,
						}),
					},
				]);
			},
		);
	});

	test("filesystem helpers surface response bodies in errors", async () => {
		setGlobalFetch(
			vi
				.fn<typeof fetch>()
				.mockResolvedValue(new Response("boom", { status: 500 })),
		);

		await expect(
			uploadFile("https://sandbox.example", "/broken.txt", "x"),
		).rejects.toThrow("Sandbox upload file failed (500): boom");
	});

	test("terminal helpers build URLs and manage websocket frames", async () => {
		Object.defineProperty(globalThis, "WebSocket", {
			configurable: true,
			writable: true,
			value: MockWebSocket,
		});

		expect(
			buildTerminalWebSocketUrl(
				"https://sandbox.example/base",
				"proc 1",
			),
		).toBe(
			"wss://sandbox.example/base/v1/processes/proc%201/terminal/ws",
		);

		const session = connectTerminal(
			"https://sandbox.example/base",
			"proc 1",
		);
		const socket = MockWebSocket.instances[0];
		expect(socket?.url).toBe(
			"wss://sandbox.example/base/v1/processes/proc%201/terminal/ws",
		);

		const outputs: Uint8Array[] = [];
		const exits: Array<{ exitCode: number | null }> = [];
		const errors: string[] = [];
		let closed = false;
		session.onData((data) => outputs.push(data));
		session.onExit((status) => exits.push(status));
		session.onError((error) => errors.push(error.message));
		session.onClose(() => {
			closed = true;
		});

		session.sendInput("ls\n");
		session.sendInput(new Uint8Array([1, 2]));
		session.resize(80, 24);

		socket?.emit("message", {
			data: new TextEncoder().encode("hello").buffer,
		});
		socket?.emit("message", {
			data: JSON.stringify({ type: "exit", exitCode: 7 }),
		});
		socket?.emit("message", {
			data: JSON.stringify({ type: "error", message: "bad terminal" }),
		});

		await vi.waitFor(() => {
			expect(outputs).toHaveLength(1);
			expect(exits).toEqual([{ exitCode: 7 }]);
			expect(errors).toContain("bad terminal");
		});

		session.close();

		expect(new TextDecoder().decode(outputs[0])).toBe("hello");
		expect(closed).toBe(true);
		expect(socket?.sent).toEqual([
			JSON.stringify({ type: "input", data: "ls\n" }),
			JSON.stringify({
				type: "input",
				data: "AQI=",
				encoding: "base64",
			}),
			JSON.stringify({ type: "resize", cols: 80, rows: 24 }),
			JSON.stringify({ type: "close" }),
		]);
	});

	test("followProcessLogs parses log SSE events and closes cleanly", async () => {
		const entries: Array<{ stream: string; data: string }> = [];
		const fetchMock = vi.fn<typeof fetch>().mockImplementation(
			async (_input, init) => {
				const stream = new ReadableStream<Uint8Array>({
					start(controller) {
						controller.enqueue(
							new TextEncoder().encode(
								[
									"event: log",
									'data: {"stream":"stdout","data":"line 1","encoding":"utf-8","sequence":1,"timestampMs":1}',
									"",
									"event: ping",
									"data: ignored",
									"",
								].join("\n"),
							),
						);
						init?.signal?.addEventListener("abort", () => {
							controller.error(
								new DOMException("aborted", "AbortError"),
							);
						});
					},
				});
				return new Response(stream, {
					status: 200,
					headers: {
						"Content-Type": "text/event-stream",
					},
				});
			},
		);
		setGlobalFetch(fetchMock);

		const subscription = await followProcessLogs(
			"https://sandbox.example/base",
			"proc-1",
			(entry) => {
				entries.push({
					stream: entry.stream,
					data: entry.data,
				});
			},
			{ stream: "stdout", tail: 5, since: 10 },
		);

		await vi.waitFor(() => {
			expect(entries).toEqual([
				{
					stream: "stdout",
					data: "line 1",
				},
			]);
		});

		subscription.close();
		await subscription.closed;

		expect(fetchMock).toHaveBeenCalledWith(
			"https://sandbox.example/base/v1/processes/proc-1/logs?follow=true&stream=stdout&tail=5&since=10",
			expect.objectContaining({
				method: "GET",
				headers: {
					Accept: "text/event-stream",
				},
			}),
		);
	});
});
