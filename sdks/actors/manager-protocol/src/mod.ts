import { ActorQuery } from "./query.ts";

export interface ActorsRequest {
	query: ActorQuery,
}

export interface ActorsResponse {
	endpoint: string,
}
