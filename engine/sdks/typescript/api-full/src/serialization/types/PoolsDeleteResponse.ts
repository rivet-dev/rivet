import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const PoolsDeleteResponse: core.serialization.Schema<serializers.PoolsDeleteResponse.Raw, Rivet.PoolsDeleteResponse> = core.serialization.object({});

export declare namespace PoolsDeleteResponse {
    export type Raw = Record<string, never>;
}
