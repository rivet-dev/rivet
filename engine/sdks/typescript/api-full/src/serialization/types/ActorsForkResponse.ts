import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { Actor } from "./Actor";

export const ActorsForkResponse: core.serialization.ObjectSchema<serializers.ActorsForkResponse.Raw, Rivet.ActorsForkResponse> = core.serialization.object({
    actor: Actor,
});

export declare namespace ActorsForkResponse {
    export interface Raw {
        actor: Actor.Raw;
    }
}
