import {
	faCheck,
	faSpinnerThird,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import type { Provider } from "@rivetkit/shared-data";
import { useQuery } from "@tanstack/react-query";
import { AnimatePresence, motion } from "framer-motion";
import { useEffect } from "react";
import { useController, useWatch } from "react-hook-form";
import { match, P } from "ts-pattern";
import { useDebounceValue } from "usehooks-ts";
import z from "zod";
import * as z4 from "zod/v4";
import { cn, Uptime } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

const IPV4_REGEX = /^(\d{1,3}\.){3}\d{1,3}$/;
const IPV6_REGEX = /^\[[\da-fA-F:]+\]$/;

function isValidHost(hostname: string): boolean {
	if (hostname === "localhost") return true;
	if (IPV4_REGEX.test(hostname)) return true;
	if (IPV6_REGEX.test(hostname)) return true;
	return z4.regexes.domain.test(hostname);
}

export const endpointSchema = z
	.string()
	.refine((val) => {
		if (!val) return false;
		const urlStr = /^https?:\/\//.test(val) ? val : `https://${val}`;
		try {
			const url = new URL(urlStr);
			if (!/^https?:$/.test(url.protocol)) return false;
			return isValidHost(url.hostname);
		} catch {
			return false;
		}
	}, "Invalid URL")
	.transform((val) => {
		if (!/^https?:\/\//.test(val)) {
			return `https://${val}`;
		}
		return val;
	});

interface ServerlessConnectionCheckProps {
	provider: Provider;
	/** How often to poll the runner health endpoint. */
	pollIntervalMs?: number;
}

export function ServerlessConnectionCheck({
	provider,
	pollIntervalMs = 3_000,
}: ServerlessConnectionCheckProps) {
	const dataProvider = useEngineCompatDataProvider();

	const endpoint: string = useWatch({ name: "endpoint" });
	const headers: [string, string][] = useWatch({ name: "headers" });

	const parsedEndpoint = endpointSchema.safeParse(endpoint);

	const enabled = Boolean(endpoint) && parsedEndpoint.success;

	const [debouncedEndpoint] = useDebounceValue(
		parsedEndpoint.success ? parsedEndpoint.data : "",
		300,
	);
	const [debouncedHeaders] = useDebounceValue(headers, 300);

	const { data, isLoading, isRefetching } = useQuery({
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
	const isError = (!isSuccess && !isLoading) || data?.failure;

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
						"text-center text-muted-foreground text-sm overflow-hidden flex items-center justify-center transition-colors border rounded-md p-4",
						isSuccess && "text-primary-foreground border-primary",
						isError &&
							"text-destructive-foreground border-destructive ",
					)}
					initial={{ minHeight: 0, height: 0, opacity: 0.5 }}
					animate={{ minHeight: "8rem", height: "auto", opacity: 1 }}
				>
					{isSuccess ? (
						<>
							<Icon
								icon={faCheck}
								className="mr-1.5 text-primary"
							/>
							{match(provider)
								.with("railway", () => "Railway")
								.with("vercel", () => "Vercel")
								.with("aws-ecs", () => "AWS ECS")
								.with(
									"cloudflare-workers",
									() => "Cloudflare Worker",
								)
								.with("gcp-cloud-run", () => "GCP Cloud Run")
								.with("hetzner", () => "Hetzner")
								.with("kubernetes", () => "Kubernetes")
								.with("custom", () => "VM & Bare Metal")
								.with(
									"custom-platform",
									() => "Custom Platform",
								)
								.exhaustive()}{" "}
							is running with RivetKit {data.success.version}
						</>
					) : !isLoading ? (
						<div className="flex flex-col items-center gap-2">
							<p className="flex items-center">
								<Icon
									icon={faTriangleExclamation}
									className="mr-1.5 text-destructive"
								/>
								Health check failed, verify the endpoint is
								correct.
							</p>
							<p>
								Endpoint:{" "}
								<a
									className="underline"
									target="_blank"
									rel="noopener noreferrer"
									href={debouncedEndpoint}
								>
									{debouncedEndpoint}
								</a>
							</p>
							{isRivetHealthCheckFailureResponse(
								data?.failure,
							) ? (
								<HealthCheckFailure error={data.failure} />
							) : null}
							<p className="text-xs text-muted-foreground">
								<Icon
									icon={faSpinnerThird}
									className="mr-1.5 animate-spin"
								/>
								{isRefetching ? (
									<span>Checking...</span>
								) : (
									<span>
										Checking again in{" "}
										<Uptime
											createTs={
												new Date(
													Date.now() +
														pollIntervalMs +
														1000,
												)
											}
											showSeconds
											absolute
										/>
									</span>
								)}
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
