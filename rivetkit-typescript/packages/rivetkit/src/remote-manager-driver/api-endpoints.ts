import { serializeActorKey } from "@/actor/keys";
import type { ClientConfig } from "@/client/client";
import type { MetadataResponse } from "@/common/router";
import type {
	ActorsCreateRequest,
	ActorsCreateResponse,
	ActorsDeleteResponse,
	ActorsGetOrCreateRequest,
	ActorsGetOrCreateResponse,
	ActorsListResponse,
} from "@/manager-api/actors";
import type { RivetId } from "@/manager-api/common";
import { apiCall } from "./api-utils";

// MARK: Get actor
export async function getActor(
	config: ClientConfig,
	_: string,
	actorId: RivetId,
): Promise<ActorsListResponse> {
	return apiCall<never, ActorsListResponse>(
		config,
		"GET",
		`/actors?actor_ids=${encodeURIComponent(actorId)}`,
	);
}

// MARK: Get actor by key
export async function getActorByKey(
	config: ClientConfig,
	name: string,
	key: string[],
): Promise<ActorsListResponse> {
	const serializedKey = serializeActorKey(key);
	return apiCall<never, ActorsListResponse>(
		config,
		"GET",
		`/actors?name=${encodeURIComponent(name)}&key=${encodeURIComponent(serializedKey)}`,
	);
}

// MARK: List actors by name
export async function listActorsByName(
	config: ClientConfig,
	name: string,
): Promise<ActorsListResponse> {
	return apiCall<never, ActorsListResponse>(
		config,
		"GET",
		`/actors?name=${encodeURIComponent(name)}`,
	);
}

// MARK: Get or create actor by id
export async function getOrCreateActor(
	config: ClientConfig,
	request: ActorsGetOrCreateRequest,
): Promise<ActorsGetOrCreateResponse> {
	return apiCall<ActorsGetOrCreateRequest, ActorsGetOrCreateResponse>(
		config,
		"PUT",
		`/actors`,
		request,
	);
}

// MARK: Create actor
export async function createActor(
	config: ClientConfig,
	request: ActorsCreateRequest,
): Promise<ActorsCreateResponse> {
	return apiCall<ActorsCreateRequest, ActorsCreateResponse>(
		config,
		"POST",
		`/actors`,
		request,
	);
}

// MARK: Destroy actor
export async function destroyActor(
	config: ClientConfig,
	actorId: RivetId,
): Promise<ActorsDeleteResponse> {
	return apiCall<never, ActorsDeleteResponse>(
		config,
		"DELETE",
		`/actors/${encodeURIComponent(actorId)}`,
	);
}

// MARK: Get metadata
export async function getMetadata(
	config: ClientConfig,
): Promise<MetadataResponse> {
	return apiCall<never, MetadataResponse>(config, "GET", `/metadata`);
}

// MARK: Get datacenters
export interface DatacentersResponse {
	datacenters: { name: string }[];
}

export async function getDatacenters(
	config: ClientConfig,
): Promise<DatacentersResponse> {
	return apiCall<never, DatacentersResponse>(config, "GET", `/datacenters`);
}

// MARK: Update runner config
export interface RunnerConfigRequest {
	datacenters: Record<
		string,
		{
			serverless: {
				url: string;
				headers: Record<string, string>;
				max_runners: number;
				min_runners: number;
				request_lifespan: number;
				runners_margin: number;
				slots_per_runner: number;
			};
		}
	>;
}

export async function updateRunnerConfig(
	config: ClientConfig,
	runnerName: string,
	request: RunnerConfigRequest,
): Promise<void> {
	return apiCall<RunnerConfigRequest, void>(
		config,
		"PUT",
		`/runner-configs/${runnerName}`,
		request,
	);
}
