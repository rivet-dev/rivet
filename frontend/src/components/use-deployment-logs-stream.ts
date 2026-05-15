import type { Rivet, RivetSse } from "@rivet-gg/cloud";
import { startTransition, useCallback, useEffect, useRef, useState } from "react";
import { cloudEnv } from "@/lib/env";

const MAX_RETRIES = 8;
const BASE_DELAY_MS = 1_000;

function parseSseEvent(raw: string): RivetSse.LogStreamEvent | null {
	let eventType = "message";
	let data = "";
	for (const line of raw.split("\n")) {
		if (line.startsWith("event:")) eventType = line.slice(6).trim();
		else if (line.startsWith("data:")) data = line.slice(5).trim();
	}
	if (!data) return null;
	try {
		const parsed = JSON.parse(data);
		if (eventType === "log") return { event: "log", data: parsed };
		if (eventType === "error") return { event: "error", data: parsed };
		if (eventType === "end") return { event: "end", data: parsed };
		if (eventType === "connected")
			return { event: "connected", data: parsed };
	} catch {
		// ignore malformed events
	}
	return null;
}

async function* streamLogsWithCredentials(
	baseUrl: string,
	project: string,
	namespace: string,
	pool: string,
	request: {
		region?: string;
		contains?: string;
		abortSignal?: AbortSignal;
	},
): AsyncGenerator<RivetSse.LogStreamEvent> {
	const params = new URLSearchParams();
	if (request.region) params.set("region", request.region);
	if (request.contains) params.set("contains", request.contains);
	const qs = params.toString();
	const url = `${baseUrl}/projects/${encodeURIComponent(project)}/namespaces/${encodeURIComponent(namespace)}/managed-pools/${encodeURIComponent(pool)}/logs${qs ? `?${qs}` : ""}`;

	const response = await fetch(url, {
		method: "GET",
		headers: {
			Accept: "text/event-stream",
			"Cache-Control": "no-cache",
		},
		credentials: "include",
		signal: request.abortSignal,
	});

	if (!response.ok) {
		const body = await response.text();
		throw new Error(
			`streamLogs request failed with status ${response.status}: ${body}`,
		);
	}
	if (!response.body) throw new Error("streamLogs: response body is null");

	const reader = response.body.getReader();
	const decoder = new TextDecoder();
	let buffer = "";
	try {
		while (true) {
			const { done, value } = await reader.read();
			if (done) break;
			buffer += decoder.decode(value, { stream: true });
			const parts = buffer.split("\n\n");
			buffer = parts.pop() ?? "";
			for (const part of parts) {
				const event = parseSseEvent(part);
				if (event != null) {
					yield event;
					if (event.event === "end") return;
				}
			}
		}
		if (buffer.trim()) {
			const event = parseSseEvent(buffer);
			if (event != null) yield event;
		}
	} finally {
		reader.releaseLock();
	}
}

async function sleep(ms: number, signal: AbortSignal) {
	return new Promise<void>((resolve) => {
		const timeout = setTimeout(resolve, ms);
		signal.addEventListener(
			"abort",
			() => {
				clearTimeout(timeout);
				resolve();
			},
			{ once: true },
		);
	});
}


const HISTORY_PAGE_SIZE = 100;
const INITIAL_HISTORY_SIZE = 50;

async function fetchLogsHistory(
	baseUrl: string,
	project: string,
	namespace: string,
	pool: string,
	params: { before?: string; limit?: number; region?: string; contains?: string },
): Promise<Rivet.LogHistoryResponseItem[]> {
	const qs = new URLSearchParams();
	if (params.before) qs.set("before", params.before);
	if (params.limit) qs.set("limit", String(params.limit));
	if (params.region) qs.set("region", params.region);
	if (params.contains) qs.set("contains", params.contains);
	const query = qs.toString();
	const url = `${baseUrl}/projects/${encodeURIComponent(project)}/namespaces/${encodeURIComponent(namespace)}/managed-pools/${encodeURIComponent(pool)}/logs/history${query ? `?${query}` : ""}`;

	const response = await fetch(url, {
		method: "GET",
		headers: { Accept: "application/json" },
		credentials: "include",
	});

	if (!response.ok) {
		const body = await response.text();
		throw new Error(`fetchLogsHistory failed with status ${response.status}: ${body}`);
	}

	return response.json();
}

function historyToLogEvent(item: Rivet.LogHistoryResponseItem): RivetSse.LogStreamEvent.Log {
	return { event: "log", data: item };
}

async function streamWithRetry(
	project: string,
	namespace: string,
	pool: string,
	filter: string | undefined,
	region: string | undefined,
	signal: AbortSignal,
	onConnected: () => void,
	onEntry: (entry: RivetSse.LogStreamEvent.Log) => void,
): Promise<"exhausted" | "ended" | "aborted" | { error: string }> {
	for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
		if (signal.aborted) return "aborted";

		try {
			const stream = streamLogsWithCredentials(
				cloudEnv().VITE_APP_CLOUD_API_URL,
				project,
				namespace,
				pool,
				{
					contains: filter || undefined,
					region: region || undefined,
					abortSignal: signal,
				},
			);

			for await (const event of stream) {
				if (event.event === "connected") {
					onConnected();
				} else if (event.event === "end") {
					return "ended";
				} else if (event.event === "error") {
					return { error: event.data.message };
				} else if (event.event === "log") {
					onEntry(event);
				}
			}
		} catch (err) {
			if ((err as Error).name === "AbortError") return "aborted";
			console.error(`Log stream error (attempt ${attempt + 1}):`, err);
		}

		if (attempt < MAX_RETRIES) {
			await sleep(BASE_DELAY_MS * 2 ** attempt, signal);
		}
	}

	return "exhausted";
}

interface UseDeploymentLogsStreamOptions {
	project: string;
	namespace: string;
	pool: string;
	filter?: string;
	region?: string;
	paused?: boolean;
}

export function useDeploymentLogsStream({
	project,
	namespace,
	pool,
	filter,
	region,
	paused = false,
}: UseDeploymentLogsStreamOptions) {
	const [logs, setLogs] = useState<RivetSse.LogStreamEvent.Log[]>([]);
	const [isLoading, setIsLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [isLoadingMore, setIsLoadingMore] = useState(false);
	const [hasMore, setHasMore] = useState(true);
	const pendingRef = useRef<RivetSse.LogStreamEvent.Log[]>([]);
	const pausedRef = useRef(paused);
	const logsRef = useRef(logs);
	logsRef.current = logs;

	useEffect(() => {
		pausedRef.current = paused;
	}, [paused]);

	useEffect(() => {
		setLogs([]);
		setIsLoading(true);
		setError(null);
		pendingRef.current = [];

		const controller = new AbortController();

		function onEntry(entry: RivetSse.LogStreamEvent.Log) {
			setIsLoading(false);
			pendingRef.current.push(entry);
			if (!pausedRef.current) {
				const toFlush = pendingRef.current;
				pendingRef.current = [];
				startTransition(() => {
					setLogs((prev) => [...prev, ...toFlush]);
				});
			}
		}

		async function start() {
			// Seed the view with recent historical logs so it isn't empty on load.
			try {
				const initial = await fetchLogsHistory(
					cloudEnv().VITE_APP_CLOUD_API_URL,
					project,
					namespace,
					pool,
					{
						limit: INITIAL_HISTORY_SIZE,
						region: region || undefined,
						contains: filter || undefined,
					},
				);
				if (controller.signal.aborted) return;
				if (initial.length > 0) {
					const converted = initial.map(historyToLogEvent);
					startTransition(() => {
						setLogs(converted);
					});
				}
			} catch {
				// Non-fatal. The stream will still start.
			}

			if (controller.signal.aborted) return;
			setIsLoading(false);

			const result = await streamWithRetry(
				project,
				namespace,
				pool,
				filter,
				region,
				controller.signal,
				() => setIsLoading(false),
				onEntry,
			);

			setIsLoading(false);
			if (result === "exhausted") {
				setError(
					"Failed to connect to log stream after multiple attempts.",
				);
			} else if (typeof result === "object") {
				setError(result.error);
			}
		}

		start().catch((err) => {
			if ((err as Error).name !== "AbortError") {
				console.error("Log stream fatal error:", err);
				setIsLoading(false);
				setError(
					"An unexpected error occurred while streaming logs.",
				);
			}
		});

		return () => controller.abort();
	}, [project, namespace, pool, filter, region]);

	useEffect(() => {
		if (!paused && pendingRef.current.length > 0) {
			const toFlush = pendingRef.current;
			pendingRef.current = [];
			startTransition(() => {
				setLogs((prev) => [...prev, ...toFlush]);
			});
		}
	}, [paused]);

	// Reset hasMore when filters change.
	useEffect(() => {
		setHasMore(true);
	}, [project, namespace, pool, filter, region]);

	const loadMoreHistory = useCallback(async () => {
		if (isLoadingMore || !hasMore) return;
		setIsLoadingMore(true);
		try {
			const currentLogs = logsRef.current;
			const before = currentLogs.length > 0
				? currentLogs[0].data.timestamp
				: new Date().toISOString();

			const items = await fetchLogsHistory(
				cloudEnv().VITE_APP_CLOUD_API_URL,
				project,
				namespace,
				pool,
				{
					before,
					limit: HISTORY_PAGE_SIZE,
					region: region || undefined,
					contains: filter || undefined,
				},
			);

			if (items.length < HISTORY_PAGE_SIZE) {
				setHasMore(false);
			}

			if (items.length > 0) {
				const converted = items.map(historyToLogEvent);
				startTransition(() => {
					setLogs((prev) => [...converted, ...prev]);
				});
			}
		} catch (err) {
			console.error("Failed to load historical logs:", err);
		} finally {
			setIsLoadingMore(false);
		}
	}, [isLoadingMore, hasMore, project, namespace, pool, filter, region]);

	return {
		logs,
		isLoading,
		error,
		streamError: null,
		isLoadingMore,
		hasMore,
		loadMoreHistory,
	};
}
