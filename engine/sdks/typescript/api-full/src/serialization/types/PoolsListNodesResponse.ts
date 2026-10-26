import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { Node } from "./Node";
import { Pagination } from "./Pagination";

export const PoolsListNodesResponse: core.serialization.ObjectSchema<serializers.PoolsListNodesResponse.Raw, Rivet.PoolsListNodesResponse> = core.serialization.object({
    nodes: core.serialization.list(Node),
    pagination: Pagination,
});

export declare namespace PoolsListNodesResponse {
    export interface Raw {
        nodes: (Node.Raw)[];
        pagination: Pagination.Raw;
    }
}
