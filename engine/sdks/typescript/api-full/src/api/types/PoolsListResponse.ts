import * as Rivet from "../index";

export interface PoolsListResponse {
    pagination: Rivet.Pagination;
    pools: (Rivet.Pool)[];
}
