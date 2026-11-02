import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const ActorsDeleteResponse: core.serialization.Schema<serializers.ActorsDeleteResponse.Raw, Rivet.ActorsDeleteResponse> = core.serialization.object({});

export declare namespace ActorsDeleteResponse {
    export type Raw = Record<string, never>;
}
