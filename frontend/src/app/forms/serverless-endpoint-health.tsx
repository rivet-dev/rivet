import {
	faCheck,
	faSpinnerThird,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import {
	createContext,
	type ReactNode,
	useCallback,
	useContext,
	useEffect,
	useId,
	useMemo,
	useState,
} from "react";
import { useWatch } from "react-hook-form";
import { useDebounceValue } from "usehooks-ts";
import z from "zod";
import { WithTooltip } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { endpointSchema } from "@/app/serverless-connection-check";

type Status = "idle" | "loading" | "success" | "error";

interface ContextValue {
	statuses: Record<string, Status>;
	setStatus: (id: string, status: Status) => void;
}

const EndpointHealthCheckContext = createContext<ContextValue | null>(null);

export function EndpointHealthCheckProvider({
	children,
}: {
	children: ReactNode;
}) {
	const [statuses, setStatuses] = useState<Record<string, Status>>({});
	const setStatus = useCallback((id: string, status: Status) => {
		setStatuses((prev) => {
			if (status === "idle") {
				if (!(id in prev)) return prev;
				const { [id]: _, ...rest } = prev;
				return rest;
			}
			if (prev[id] === status) return prev;
			return { ...prev, [id]: status };
		});
	}, []);
	const value = useMemo(() => ({ statuses, setStatus }), [statuses, setStatus]);
	return (
		<EndpointHealthCheckContext.Provider value={value}>
			{children}
		</EndpointHealthCheckContext.Provider>
	);
}

export function useEndpointHealthChecksValid() {
	const ctx = useContext(EndpointHealthCheckContext);
	if (!ctx) return true;
	return Object.values(ctx.statuses).every((s) => s === "success");
}

export function useEndpointHealthChecksLoading() {
	const ctx = useContext(EndpointHealthCheckContext);
	if (!ctx) return false;
	return Object.values(ctx.statuses).some((s) => s === "loading");
}

interface EndpointHealthIndicatorProps {
	endpointName: string;
	headersName?: string;
	enabledName?: string;
	pollIntervalMs?: number;
}

export function EndpointHealthIndicator({
	endpointName,
	headersName,
	enabledName,
	pollIntervalMs = 5_000,
}: EndpointHealthIndicatorProps) {
	const id = useId();
	const setStatus = useContext(EndpointHealthCheckContext)?.setStatus;
	const dataProvider = useEngineCompatDataProvider();

	const endpointRaw = useWatch({ name: endpointName }) as string | undefined;
	const headersRaw = useWatch({ name: headersName ?? "" }) as
		| [string, string][]
		| undefined;
	const fieldEnabled = useWatch({ name: enabledName ?? "" }) as
		| boolean
		| undefined;
	const isFieldActive = enabledName ? fieldEnabled !== false : true;

	const endpoint = endpointRaw ?? "";
	const parsed = endpointSchema.safeParse(endpoint);
	const enabled = isFieldActive && Boolean(endpoint) && parsed.success;

	const [debouncedEndpoint] = useDebounceValue(
		parsed.success ? parsed.data : "",
		500,
	);
	const [debouncedHeaders] = useDebounceValue(headersRaw, 500);

	const { data, isLoading, isError, error } = useQuery({
		...dataProvider.runnerHealthCheckQueryOptions({
			runnerUrl: debouncedEndpoint,
			headers: Object.fromEntries(
				(debouncedHeaders || [])
					.filter(([k, v]) => k && v)
					.map(([k, v]) => [k, v]),
			),
		}),
		enabled,
		retry: 0,
		refetchInterval: pollIntervalMs,
	});

	const isSuccess = !!(data && "success" in data && data.success);
	const isFailure =
		!!(data && "failure" in data && data.failure) || isError;
	const status: Status = !enabled
		? "idle"
		: isLoading
			? "loading"
			: isSuccess
				? "success"
				: isFailure
					? "error"
					: "loading";

	useEffect(() => {
		setStatus?.(id, status);
		return () => setStatus?.(id, "idle");
	}, [setStatus, id, status]);

	if (!enabled) return null;

	if (status === "loading") {
		return (
			<div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
				<Icon
					icon={faSpinnerThird}
					className="animate-spin text-muted-foreground"
				/>
			</div>
		);
	}

	if (status === "success") {
		return (
			<div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
				<Icon icon={faCheck} className="text-primary" />
			</div>
		);
	}

	if (status === "error") {
		return (
			<div className="absolute right-3 top-1/2 -translate-y-1/2">
				<WithTooltip

					delayDuration={0}
					trigger={
						<span className="cursor-help">
							<Icon
								icon={faTriangleExclamation}
								className="text-destructive"
							/>
						</span>
					}
					content={extractErrorMessage(data, error)}
				/>
			</div>
		);
	}

	return null;
}

const failureSchema = z.object({
	failure: z.object({
		error: z.object({
			message: z.string().optional(),
			details: z.string().optional(),
			metadata: z
				.object({
					kind: z.string().optional(),
					status_code: z.number().optional(),
				})
				.partial()
				.optional(),
		}),
	}),
});

function extractErrorMessage(data: unknown, error: unknown): string {
	const fallback = "Health check failed. Verify the endpoint is reachable.";
	const parsed = failureSchema.safeParse(data);
	if (parsed.success) {
		const { message, details, metadata } = parsed.data.failure.error;
		if (message) {
			return details ? `${message} (${details})` : message;
		}
		if (metadata?.kind && metadata.status_code) {
			return `${metadata.kind.replace(/_/g, " ")} (HTTP ${metadata.status_code})`;
		}
	}
	if (error instanceof Error && error.message) {
		return error.message.slice(0, 200);
	}
	return fallback;
}
