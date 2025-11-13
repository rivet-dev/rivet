import pRetry from "p-retry";
import type { ClientConfig } from "@/client/client";
import type { MetadataResponse } from "@/common/router";
import { stringifyError } from "@/common/utils";
import { getMetadata } from "./api-endpoints";
import { getEndpoint } from "./api-utils";
import { logger } from "./log";

// Global cache to store metadata check promises for each endpoint
const metadataLookupCache = new Map<string, Promise<MetadataResponse>>();

export async function lookupMetadataCached(
	config: ClientConfig,
): Promise<MetadataResponse> {
	const endpoint = getEndpoint(config);

	// Check if metadata lookup is already in progress or completed for this endpoint
	const existingPromise = metadataLookupCache.get(endpoint);
	if (existingPromise) {
		return existingPromise;
	}

	// Create and store the promise immediately to prevent racing requests
	const metadataLookupPromise = pRetry(
		async () => {
			logger().debug({
				msg: "fetching metadata",
				endpoint,
			});

			const metadataData = await getMetadata(config);

			logger().debug({
				msg: "received metadata",
				endpoint,
				clientEndpoint: metadataData.clientEndpoint,
			});

			return metadataData;
		},
		{
			forever: true,
			minTimeout: 500,
			maxTimeout: 15_000,
			onFailedAttempt: (error) => {
				logger().warn({
					msg: "failed to fetch metadata, retrying",
					endpoint,
					attempt: error.attemptNumber,
					error: stringifyError(error),
				});
			},
		},
	);

	metadataLookupCache.set(endpoint, metadataLookupPromise);
	return metadataLookupPromise;
}
