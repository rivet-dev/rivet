import type { ActorQuery } from "./query.ts";

export interface ActorsRequest {
	query: ActorQuery;
}

export interface ActorsResponse {
	endpoint: string;
}

export interface RivetConfigResponse {
	endpoint: string;
	project?: string;
	environment?: string;
}
