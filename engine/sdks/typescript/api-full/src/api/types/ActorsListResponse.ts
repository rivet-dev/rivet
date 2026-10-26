import * as Rivet from "../index";

export interface ActorsListResponse {
    actors: (Rivet.Actor)[];
    pagination: Rivet.Pagination;
}
