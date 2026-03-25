/**
 * Cloud client factory.
 *
 * Creates a typed RivetClient from @rivet-gg/cloud for communicating with the
 * Rivet Cloud REST API (https://cloud-api.rivet.dev).
 */

import { RivetClient, RivetError } from "@rivet-gg/cloud";

export { RivetClient, RivetError };

export function createCloudClient(opts: {
	token: string;
	baseUrl?: string;
}): RivetClient {
	return new RivetClient({
		environment: "",
		baseUrl: opts.baseUrl ?? "https://cloud-api.rivet.dev",
		token: opts.token,
	});
}

/** Docker registry credentials (not yet part of the SDK — use token-based auth). */
export interface DockerCredentials {
	registryUrl: string;
	username: string;
	password: string;
}

