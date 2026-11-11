import * as Rivet from "../../index";

/**
 * @example
 *     {
 *         namespace: "namespace",
 *         key: "key"
 *     }
 */
export interface ActorsForkRequest {
    namespace?: string;
    /** Key of the new actor in canonical slash-separated form. Omit for an actor without a key. */
    key?: string;
    /** Fork input as standard base64, passed to the new actor's `onFork` hook. It comes from the caller, so `onFork` must validate it like any client input. */
    input?: string;
}
