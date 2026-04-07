import type { RivetSse } from "@rivet-gg/cloud";
import { startTransition, useEffect, useRef, useState } from "react";
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
	const pendingRef = useRef<RivetSse.LogStreamEvent.Log[]>([]);
	const pausedRef = useRef(paused);

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

		streamWithRetry(
			project,
			namespace,
			pool,
			filter,
			region,
			controller.signal,
			() => setIsLoading(false),
			onEntry,
		)
			.then((result) => {
				setIsLoading(false);
				if (result === "exhausted") {
					setError(
						"Failed to connect to log stream after multiple attempts.",
					);
				} else if (typeof result === "object") {
					setError(result.error);
				}
			})
			.catch((err) => {
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

	return {
		logs,
		isLoading,
		error,
		streamError: null,
		isLoadingMore: false,
		hasMore: false,
		loadMoreHistory: () => { },
	};
}
