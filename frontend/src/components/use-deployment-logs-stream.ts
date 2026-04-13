import type { RivetSse } from "@rivet-gg/cloud";
import { startTransition, useEffect, useRef, useState } from "react";
import { clerk } from "@/lib/auth";
import { cloudEnv } from "@/lib/env";

// Lazy-load RivetSse at runtime. The @rivet-gg/cloud package may not export
// this in all versions (e.g. pkg.pr.new previews), and this file is only used
// by cloud routes, never the engine UI. The dynamic import prevents rollup from
// failing at build time when the named export is missing.
let _rivetSse: typeof RivetSse | null = null;
async function getRivetSse(): Promise<typeof RivetSse> {
	if (!_rivetSse) {
		const mod = await import("@rivet-gg/cloud");
		if (!("RivetSse" in mod)) {
			throw new Error(
				"@rivet-gg/cloud does not export RivetSse — this feature requires a cloud SDK version that includes SSE support",
			);
		}
		_rivetSse = (mod as { RivetSse: typeof RivetSse }).RivetSse;
	}
	return _rivetSse;
}

const MAX_RETRIES = 8;
const BASE_DELAY_MS = 1_000;

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
	const options = {
		baseUrl: cloudEnv().VITE_APP_CLOUD_API_URL,
		environment: "",
		token: async () => (await clerk.session?.getToken()) || "",
	};

	for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
		if (signal.aborted) return "aborted";

		try {
			const sse = await getRivetSse();
			const stream = sse.streamLogs(
				options,
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

	return { logs, isLoading, error };
}
