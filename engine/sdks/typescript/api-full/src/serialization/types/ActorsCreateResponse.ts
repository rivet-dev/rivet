import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { Actor } from "./Actor";

export const ActorsCreateResponse: core.serialization.ObjectSchema<serializers.ActorsCreateResponse.Raw, Rivet.ActorsCreateResponse> = core.serialization.object({
    actor: Actor,
    created: core.serialization.boolean(),
});

export declare namespace ActorsCreateResponse {
    export interface Raw {
        actor: Actor.Raw;
        created: boolean;
    }
}
