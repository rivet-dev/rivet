import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { ActorName } from "./ActorName";

export const ActorsListNamesResponse: core.serialization.ObjectSchema<serializers.ActorsListNamesResponse.Raw, Rivet.ActorsListNamesResponse> = core.serialization.object({
    names: core.serialization.record(core.serialization.string(), ActorName),
});

export declare namespace ActorsListNamesResponse {
    export interface Raw {
        names: Record<string, ActorName.Raw>;
    }
}
