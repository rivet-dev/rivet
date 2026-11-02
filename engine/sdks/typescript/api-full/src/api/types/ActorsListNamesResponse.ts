import * as Rivet from "../index";

export interface ActorsListNamesResponse {
    /** Map from actor name to its metadata. */
    names: Record<string, Rivet.ActorName>;
}
