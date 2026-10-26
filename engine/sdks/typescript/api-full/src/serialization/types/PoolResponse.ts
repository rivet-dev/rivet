import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { Pool } from "./Pool";

export const PoolResponse: core.serialization.ObjectSchema<serializers.PoolResponse.Raw, Rivet.PoolResponse> = core.serialization.object({
    pool: Pool,
});

export declare namespace PoolResponse {
    export interface Raw {
        pool: Pool.Raw;
    }
}
