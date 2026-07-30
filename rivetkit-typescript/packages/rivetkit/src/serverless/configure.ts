import { convertRegistryConfigToClientConfig } from "@/client/config";
import type { ClientConfig } from "@/client/config";
import { stringifyError } from "@/common/utils";
import { RivetError } from "@/actor/errors";
import {
	getDatacenters,
	getRunnerConfig,
	updateRunnerConfig,
} from "@/engine-client/api-endpoints";
import type { RegistryConfig } from "@/registry/config";
import { logger } from "@/registry/log";
import { isDev } from "@/utils/env-vars";

const DEFAULT_CONFIGURE_TIMEOUT_MS = 60_000;
const CONFIGURE_RETRY_DELAY_MS = 1_000;
const LOCAL_HANDLER_PROBE_TIMEOUT_MS = 1_000;

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function configureTimeoutMs() {
	const value = process.env.RIVET_SERVERLESS_CONFIGURE_TIMEOUT_MS;
	if (value === undefined || value === "")
		return DEFAULT_CONFIGURE_TIMEOUT_MS;

	const parsed = Number(value);
	if (!Number.isFinite(parsed) || parsed < 0) {
		throw new Error(
			"RIVET_SERVERLESS_CONFIGURE_TIMEOUT_MS must be a finite non-negative number",
		);
	}

	return parsed;
}

export async function configureServerlessPool(
	config: RegistryConfig,
): Promise<void> {
	logger().debug({ msg: "configuring serverless pool" });

	const startedAt = Date.now();
	const timeoutMs = configureTimeoutMs();
	let attempts = 0;
	let lastError: unknown;

	while (Date.now() - startedAt <= timeoutMs) {
		attempts += 1;
		try {
			if (!config.namespace) {
				throw new Error(
					"namespace is required for serverless configuration",
				);
			}
			if (!config.endpoint) {
				throw new Error(
					"endpoint is required for serverless configuration",
				);
			}
			if (!config.configurePool) {
				throw new Error(
					"configurePool is required for serverless configuration",
				);
			}

			const customConfig = config.configurePool;
			const clientConfig = convertRegistryConfigToClientConfig(config);
			const dcsRes = await getDatacenters(clientConfig);
			const poolName = customConfig.name ?? "default";
			await assertLocalServerlessHandlerOwnership(
				clientConfig,
				poolName,
				customConfig.url,
			);
			const serverlessToken = config.token ?? config.publicToken;
			const headers = {
				...(serverlessToken
					? { "x-rivet-token": serverlessToken }
					: {}),
				...(customConfig.headers ?? {}),
			};
			const serverlessConfig = {
				serverless: {
					url: customConfig.url,
					headers,
					request_lifespan: customConfig.requestLifespan ?? 60 * 60,
					drain_grace_period: customConfig.drainGracePeriod,
					metadata_poll_interval:
						customConfig.metadataPollInterval ?? 1000,
					max_runners: 100_000,
					min_runners: 0,
					runners_margin: 0,
					slots_per_runner: 1,
				},
				metadata: customConfig.metadata ?? {},
				drain_on_version_upgrade:
					customConfig.drainOnVersionUpgrade ?? true,
			};

			await updateRunnerConfig(clientConfig, poolName, {
				datacenters: Object.fromEntries(
					dcsRes.datacenters.map((dc) => [dc.name, serverlessConfig]),
				),
			});

			logger().info({
				msg: "serverless pool configured successfully",
				poolName,
				namespace: config.namespace,
				attempts,
			});
			return;
		} catch (error) {
			if (
				error instanceof RivetError &&
				error.group === "rivetkit" &&
				error.code === "local_serverless_handler_conflict"
			) {
				throw error;
			}
			lastError = error;
			logger().warn({
				msg: "serverless pool configuration attempt failed",
				attempts,
				error: stringifyError(error),
			});
			await sleep(CONFIGURE_RETRY_DELAY_MS);
		}
	}

	logger().error({
		msg: "failed to configure serverless pool, validate endpoint is configured correctly then restart this process",
		attempts,
		error: stringifyError(lastError),
	});
	throw lastError;
}

async function assertLocalServerlessHandlerOwnership(
	config: ClientConfig,
	poolName: string,
	requestedUrl: string,
): Promise<void> {
	if (!isDev()) return;

	const response = await getRunnerConfig(config, poolName);
	const datacenters = response.runner_configs[poolName]?.datacenters ?? {};
	const existingUrls = new Set(
		Object.values(datacenters)
			.map((runnerConfig) => runnerConfig.serverless?.url)
			.filter((url): url is string => url !== undefined),
	);

	for (const existingUrl of existingUrls) {
		if (normalizeHandlerUrl(existingUrl) === normalizeHandlerUrl(requestedUrl)) {
			continue;
		}
		if (!(await handlerIsLive(existingUrl))) continue;

		throw new RivetError(
			"rivetkit",
			"local_serverless_handler_conflict",
			`namespace \`${config.namespace}\` and pool \`${poolName}\` already use the live serverless handler ${existingUrl}. Stop the other project, set RIVET_NAMESPACE to a different namespace, or set RIVET_RUN_ENGINE_PORT to a different Engine port. See https://rivet.dev/docs/general/environment-variables/`,
			{
				public: true,
				metadata: {
					namespace: config.namespace,
					poolName,
					existingUrl,
					requestedUrl,
				},
			},
		);
	}
}

export function normalizeHandlerUrl(value: string): string {
	const url = new URL(value);
	url.hash = "";
	url.pathname = url.pathname.replace(/\/+$/, "");
	return url.toString();
}

async function handlerIsLive(handlerUrl: string): Promise<boolean> {
	const metadataUrl = new URL(handlerUrl);
	metadataUrl.pathname = `${metadataUrl.pathname.replace(/\/+$/, "")}/metadata`;
	try {
		const response = await fetch(metadataUrl, {
			signal: AbortSignal.timeout(LOCAL_HANDLER_PROBE_TIMEOUT_MS),
		});
		return response.ok;
	} catch {
		return false;
	}
}
