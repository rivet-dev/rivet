import * as Rivet from "../index";

export interface MetadataGetResponse {
    /** Always `engine`. */
    runtime: "engine";
    /** Version of the control plane. */
    version: string;
    /** Version of the protocol nodes use to connect to the control plane. */
    envoyProtocolVersion: number;
    /** Actor names the default namespace's connected nodes accept. Only present for requests made with the admin token. */
    actorNames?: Record<string, Rivet.ActorName>;
}
