import * as Rivet from "../index";

export interface PoolsListNodesResponse {
    nodes: (Rivet.Node)[];
    pagination: Rivet.Pagination;
}
