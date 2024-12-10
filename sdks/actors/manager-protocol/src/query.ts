import { RivetClient } from "@rivet-gg/api";
import { ActorTags } from "../../common/src/utils.ts";

export type ActorQuery =
	| { actorId: { actorId: string } }
	| { get: { tags: ActorTags } }
	| { getOrCreate: GetOrCreateRequest }
	| { create: CreateRequest };

export interface GetOrCreateRequest {
	tags: ActorTags;
	create?: CreateRequest;
}

// TODO(RVT-4250):
export type CreateRequest = RivetClient.actor.CreateActorRequest;
