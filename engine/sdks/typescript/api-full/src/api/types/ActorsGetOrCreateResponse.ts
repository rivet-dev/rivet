import * as Rivet from "../index";

export interface ActorsGetOrCreateResponse {
    actor: Rivet.Actor;
    /** `true` if this request created the actor, `false` if it already existed. */
    created: boolean;
}
