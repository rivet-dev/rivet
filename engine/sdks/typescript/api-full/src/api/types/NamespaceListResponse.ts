import * as Rivet from "../index";

export interface NamespaceListResponse {
    namespaces: (Rivet.Namespace)[];
    pagination: Rivet.Pagination;
}
