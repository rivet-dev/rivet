import type { Rivet } from "@rivet-gg/cloud";
import * as Sentry from "@sentry/react";
import {
	startTransition,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import { cloudEnv } from "@/lib/env";

const MAX_RETRIES = 8;
const BASE_DELAY_MS = 1_000;

function parseSseEvent(raw: string): Rivet.LogStreamEvent | null {
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
): AsyncGenerator<Rivet.LogStreamEvent> {
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

const HISTORY_PAGE_SIZE = 300;
const INITIAL_HISTORY_SIZE = 100;

async function fetchLogsHistory(
	baseUrl: string,
	project: string,
	namespace: string,
	pool: string,
	params: {
		before?: string;
		limit?: number;
		region?: string;
		contains?: string;
		signal?: AbortSignal;
	},
): Promise<Rivet.LogHistoryPaginatedResponse> {
	const qs = new URLSearchParams();
	if (params.before) qs.set("before", params.before);
	if (params.limit) qs.set("limit", String(params.limit));
	if (params.region) qs.set("region", params.region);
	if (params.contains) qs.set("contains", params.contains);
	const query = qs.toString();
	const url = `${baseUrl}/projects/${encodeURIComponent(project)}/namespaces/${encodeURIComponent(namespace)}/managed-pools/${encodeURIComponent(pool)}/logs/history-paginated${query ? `?${query}` : ""}`;

	const response = await fetch(url, {
		method: "GET",
		headers: { Accept: "application/json" },
		credentials: "include",
		signal: params.signal,
	});

	if (!response.ok) {
		const body = await response.text();
		throw new Error(
			`fetchLogsHistory failed with status ${response.status}: ${body}`,
		);
	}

	return response.json();
}

// Cloud Run emits this on every container cold-start, prefixed with its own
// timestamp on stderr. When the user hasn't pushed an image yet, surface a
// Rivet-flavored hint instead of leaky provider detail. Matched by suffix
// (after Cloud Run's leading "YYYY/MM/DD HH:MM:SS " prefix) and gated on
// `stream === "stderr"` so we don't grab a user-emitted line that happens to
// contain the same text.
const CLOUD_RUN_HELLO_SUFFIX =
	"Hello from Cloud Run! The container started successfully and is listening for HTTP requests on port 8080";
const RIVET_COMPUTE_HELLO =
	"Hello from Rivet Compute! Waiting for you to deploy an image. See rivet.dev/docs/connect/rivet-compute to learn more.";

function rewriteLogEntry<T extends { message: string; stream?: string }>(
	data: T,
): T {
	if (
		data.stream === "stderr" &&
		data.message.endsWith(CLOUD_RUN_HELLO_SUFFIX)
	) {
		return { ...data, message: RIVET_COMPUTE_HELLO };
	}
	return data;
}

function historyToLogEvent(
	item: Rivet.LogHistoryPaginatedResponse.Entries.Item,
): Rivet.LogStreamEvent.Log {
	return { event: "log", data: rewriteLogEntry(item) };
}

async function streamWithRetry(
	project: string,
	namespace: string,
	pool: string,
	filter: string | undefined,
	region: string | undefined,
	signal: AbortSignal,
	onConnected: () => void,
	onEntry: (entry: Rivet.LogStreamEvent.Log) => void,
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
					onEntry({ ...event, data: rewriteLogEntry(event.data) });
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
	initialBefore?: string;
}

export function useDeploymentLogsStream({
	project,
	namespace,
	pool,
	filter,
	region,
	paused = false,
	initialBefore,
}: UseDeploymentLogsStreamOptions) {
	const [logs, setLogs] = useState<Rivet.LogStreamEvent.Log[]>([]);
	const [isLoading, setIsLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [isLoadingMore, setIsLoadingMore] = useState(false);
	const [hasMore, setHasMore] = useState(true);
	// Oldest raw timestamp scanned so far (the boundary the next page fetches
	// before). Surfaced for the empty state so the user can see how far back the
	// search has reached when nothing matches the filter yet.
	const [oldestScannedTs, setOldestScannedTs] = useState<string | undefined>(
		undefined,
	);
	const pendingRef = useRef<Rivet.LogStreamEvent.Log[]>([]);
	const pausedRef = useRef(paused);
	// Dedupes entries that appear in both the initial history fetch and
	// the live stream (and across "load more" page boundaries that share
	// a timestamp). `insertId` is the only unique identifier the API
	// exposes per log entry.
	const seenInsertIdsRef = useRef<Set<string>>(new Set());
	// Cursor for the next older history page. The API returns this per page as the
	// oldest raw timestamp it scanned, so paging back works even across windows that
	// contain no entries matching the current filter.
	const nextCursorRef = useRef<string | undefined>(undefined);

	useEffect(() => {
		pausedRef.current = paused;
	}, [paused]);

	useEffect(() => {
		setLogs([]);
		setIsLoading(true);
		setError(null);
		setHasMore(true);
		nextCursorRef.current = undefined;
		setOldestScannedTs(undefined);
		pendingRef.current = [];
		seenInsertIdsRef.current = new Set();

		const controller = new AbortController();

		function onEntry(entry: Rivet.LogStreamEvent.Log) {
			setIsLoading(false);
			const insertId = entry.data.insertId;
			if (seenInsertIdsRef.current.has(insertId)) return;
			seenInsertIdsRef.current.add(insertId);
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
						before: initialBefore,
						limit: INITIAL_HISTORY_SIZE,
						region: region || undefined,
						contains: filter || undefined,
						signal: controller.signal,
					},
				);
				if (controller.signal.aborted) return;

				nextCursorRef.current = initial.nextCursor;
				setOldestScannedTs(initial.nextCursor);
				setHasMore(initial.hasMore);
				if (initial.entries.length > 0) {
					const converted: Rivet.LogStreamEvent.Log[] = [];
					for (const item of initial.entries) {
						seenInsertIdsRef.current.add(item.insertId);
						converted.push(historyToLogEvent(item));
					}
					startTransition(() => {
						setLogs(converted);
					});
				}
			} catch (err) {
				// The fetch was cancelled by the effect cleanup (e.g. filter
				// change or unmount). Not an error worth surfacing.
				if ((err as Error).name === "AbortError") return;
				// Non-fatal. The stream will still start.
				console.warn("Failed to fetch initial log history:", err);
				Sentry.captureException(err, {
					tags: { source: "deployment-logs-initial-history" },
					contexts: {
						logs: { project, namespace, pool, region, filter },
					},
				});
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
				setError("An unexpected error occurred while streaming logs.");
			}
		});

		return () => controller.abort();
	}, [project, namespace, pool, filter, region, initialBefore]);

	useEffect(() => {
		if (!paused && pendingRef.current.length > 0) {
			const toFlush = pendingRef.current;
			pendingRef.current = [];
			startTransition(() => {
				setLogs((prev) => [...prev, ...toFlush]);
			});
		}
	}, [paused]);

	const loadMoreHistory = useCallback(async () => {
		if (isLoadingMore || !hasMore) return;
		// Fall back to now when we have no cursor yet, e.g. the initial history
		// fetch failed. `hasMore` stays true until a successful page says otherwise,
		// so a retry still issues a request instead of silently disabling paging.
		const before = nextCursorRef.current ?? new Date().toISOString();
		setIsLoadingMore(true);
		try {
			const page = await fetchLogsHistory(
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
			nextCursorRef.current = page.nextCursor;
			setOldestScannedTs(page.nextCursor);
			setHasMore(page.hasMore);

			// Multiple entries can share a timestamp, so `seenInsertIdsRef` filters
			// out any overlap at the page boundary.
			const fresh = page.entries.filter(
				(item) => !seenInsertIdsRef.current.has(item.insertId),
			);
			for (const item of fresh) {
				seenInsertIdsRef.current.add(item.insertId);
			}
			if (fresh.length > 0) {
				const converted = fresh.map(historyToLogEvent);
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
		oldestScannedTs,
		loadMoreHistory,
	};
}
