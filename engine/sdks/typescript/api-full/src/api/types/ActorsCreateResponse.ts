import * as Rivet from "../index";

export interface ActorsCreateResponse {
    actor: Rivet.Actor;
    /** Always `true`. */
    created: boolean;
}
