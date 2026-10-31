import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const ActorsSleepResponse: core.serialization.Schema<serializers.ActorsSleepResponse.Raw, Rivet.ActorsSleepResponse> = core.serialization.object({});

export declare namespace ActorsSleepResponse {
    export type Raw = Record<string, never>;
}
