import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { Pagination } from "./Pagination";
import { Pool } from "./Pool";

export const PoolsListResponse: core.serialization.ObjectSchema<serializers.PoolsListResponse.Raw, Rivet.PoolsListResponse> = core.serialization.object({
    pagination: Pagination,
    pools: core.serialization.list(Pool),
});

export declare namespace PoolsListResponse {
    export interface Raw {
        pagination: Pagination.Raw;
        pools: (Pool.Raw)[];
    }
}
