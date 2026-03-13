import { RivetSse } from "@rivet-gg/cloud";
import { startTransition, useEffect, useRef, useState } from "react";
import { clerk } from "@/lib/auth";
import { cloudEnv } from "@/lib/env";

const MAX_RETRIES = 8;
const BASE_DELAY_MS = 1_000;

async function sleep(ms: number, signal: AbortSignal) {
	return new Promise<void>((resolve) => {
		const timeout = setTimeout(resolve, ms);
		signal.addEventListener("abort", () => {
			clearTimeout(timeout);
			resolve();
		}, { once: true });
	});
}

async function streamWithRetry(
	project: string,
	namespace: string,
	pool: string,
	filter: string | undefined,
	region: string | undefined,
	signal: AbortSignal,
	onEntry: (entry: RivetSse.LogEntry) => void,
): Promise<"exhausted" | "ended" | "aborted" | { error: string }> {
	const options = {
		baseUrl: cloudEnv().VITE_APP_CLOUD_API_URL,
		environment: "",
		token: async () => (await clerk.session?.getToken()) || "",
	};

	for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
		if (signal.aborted) return "aborted";

		try {
			const stream = RivetSse.streamLogs(options, project, namespace, pool, {
				contains: filter || undefined,
				region: region || undefined,
				abortSignal: signal,
			});

			for await (const event of stream) {
				if (event.event === "end") return "ended";
				if (event.event === "error") {
					return { error: event.data.message };
				}
				if (event.event === "log") {
					onEntry(event.data);
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
	const [logs, setLogs] = useState<RivetSse.LogEntry[]>([]);
	const [isLoading, setIsLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const pendingRef = useRef<RivetSse.LogEntry[]>([]);
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

		function onEntry(entry: RivetSse.LogEntry) {
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

		streamWithRetry(project, namespace, pool, filter, region, controller.signal, onEntry)
			.then((result) => {
				setIsLoading(false);
				if (result === "exhausted") {
					setError("Failed to connect to log stream after multiple attempts.");
				} else if (typeof result === "object") {
					setError(result.error);
				}
			})
			.catch((err) => {
				if ((err as Error).name !== "AbortError") {
					console.error("Log stream fatal error:", err);
					setIsLoading(false);
					setError("An unexpected error occurred while streaming logs.");
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

	return { logs, isLoading, error };
}
