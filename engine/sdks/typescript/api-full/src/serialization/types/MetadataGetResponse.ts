import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { ActorName } from "./ActorName";

export const MetadataGetResponse: core.serialization.ObjectSchema<serializers.MetadataGetResponse.Raw, Rivet.MetadataGetResponse> = core.serialization.object({
    runtime: core.serialization.stringLiteral("engine"),
    version: core.serialization.string(),
    envoyProtocolVersion: core.serialization.number(),
    actorNames: core.serialization.record(core.serialization.string(), ActorName).optional(),
});

export declare namespace MetadataGetResponse {
    export interface Raw {
        runtime: "engine";
        version: string;
        envoyProtocolVersion: number;
        actorNames?: Record<string, ActorName.Raw> | null;
    }
}
