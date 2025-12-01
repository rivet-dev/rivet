import {
	faCheck,
	faSpinnerThird,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import { useQuery } from "@tanstack/react-query";
import { AnimatePresence, motion } from "framer-motion";
import { useEffect } from "react";
import { useController, useWatch } from "react-hook-form";
import { match, P } from "ts-pattern";
import { useDebounceValue } from "usehooks-ts";
import z from "zod";
import { cn } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

export const endpointSchema = z
	.string()
	.nonempty("Endpoint is required")
	.url("Please enter a valid URL")
	.endsWith("/api/rivet", "Endpoint must end with /api/rivet");

interface ServerlessConnectionCheckProps {
	providerLabel: string;
	/** How often to poll the runner health endpoint. */
	pollIntervalMs?: number;
}

export function ServerlessConnectionCheck({
	providerLabel,
	pollIntervalMs = 3_000,
}: ServerlessConnectionCheckProps) {
	const dataProvider = useEngineCompatDataProvider();

	const endpoint: string = useWatch({ name: "endpoint" });
	const headers: [string, string][] = useWatch({ name: "headers" });

	const enabled =
		Boolean(endpoint) && endpointSchema.safeParse(endpoint).success;

	const [debouncedEndpoint] = useDebounceValue(endpoint, 300);
	const [debouncedHeaders] = useDebounceValue(headers, 300);

	const { isSuccess, data, isError, isRefetchError, isLoadingError, error } =
		useQuery({
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

	const {
		field: { onChange },
	} = useController({ name: "success" });

	useEffect(() => {
		onChange(isSuccess);
	}, [isSuccess, onChange]);

	return (
		<AnimatePresence>
			{enabled ? (
				<motion.div
					layoutId="serverless-health-check"
					className={cn(
						"text-center text-muted-foreground text-sm overflow-hidden flex items-center justify-center",
						isSuccess && "text-primary-foreground",
						isError && "text-destructive-foreground",
					)}
					initial={{ height: 0, opacity: 0.5 }}
					animate={{ height: "8rem", opacity: 1 }}
				>
					{isSuccess ? (
						<>
							<Icon
								icon={faCheck}
								className="mr-1.5 text-primary"
							/>
							{providerLabel} is running with RivetKit{" "}
							{(data as any)?.version}
						</>
					) : isError || isRefetchError || isLoadingError ? (
						<div className="flex flex-col items-center gap-2">
							<p className="flex items-center">
								<Icon
									icon={faTriangleExclamation}
									className="mr-1.5 text-destructive"
								/>
								Health check failed, verify the endpoint is
								correct.
							</p>
							{isRivetHealthCheckFailureResponse(error) ? (
								<HealthCheckFailure error={error} />
							) : null}
							<p>
								Endpoint{" "}
								<a
									className="underline"
									href={endpoint}
									target="_blank"
									rel="noopener noreferrer"
								>
									{endpoint}
								</a>
							</p>
						</div>
					) : (
						<div className="flex flex-col items-center gap-2">
							<div className="flex items-center">
								<Icon
									icon={faSpinnerThird}
									className="mr-1.5 animate-spin"
								/>
								Waiting for Runner to connect...
							</div>
						</div>
					)}
				</motion.div>
			) : null}
		</AnimatePresence>
	);
}

function isRivetHealthCheckFailureResponse(
	error: any,
): error is Rivet.RunnerConfigsServerlessHealthCheckResponseFailure["failure"] {
	return error && "error" in error;
}

function HealthCheckFailure({
	error,
}: {
	error: Rivet.RunnerConfigsServerlessHealthCheckResponseFailure["failure"];
}) {
	if (!("error" in error)) {
		return null;
	}
	if (!error.error) {
		return null;
	}

	return match(error.error)
		.with({ nonSuccessStatus: P.any }, (e) => {
			return (
				<p>
					Health check failed with status{" "}
					{e.nonSuccessStatus.statusCode}
				</p>
			);
		})
		.with({ invalidRequest: P.any }, () => {
			return <p>Health check failed because the request was invalid.</p>;
		})
		.with({ invalidResponseJson: P.any }, () => {
			return (
				<p>
					Health check failed because the response was not valid JSON.
				</p>
			);
		})
		.with({ requestFailed: P.any }, () => {
			return (
				<p>
					Health check failed because the request could not be
					completed.
				</p>
			);
		})
		.with({ requestTimedOut: P.any }, () => {
			return <p>Health check failed because the request timed out.</p>;
		})
		.with({ invalidResponseSchema: P.any }, () => {
			return (
				<p>Health check failed because the response was not valid.</p>
			);
		})
		.exhaustive();
}
