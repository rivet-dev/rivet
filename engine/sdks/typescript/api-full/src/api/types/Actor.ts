import * as Rivet from "../index";

export interface Actor {
    actorId: Rivet.RivetId;
    /** Actor name. */
    name: string;
    /** Actor key in slash-separated form. An empty string for an actor without a key. */
    key: string;
    /** Name of the namespace the actor belongs to. */
    namespace: string;
    /** Name of the node pool the actor runs in. */
    pool: string;
    /** When the actor was created, in Unix milliseconds. */
    createTs: number;
    /** When the actor first started. Not recorded by this version, so always `null`. */
    startTs: number | null;
    /** When the actor started waiting for a node. Not recorded by this version, so always `null`. */
    pendingAllocationTs: number | null;
    /** When the actor last became connectable. Not recorded by this version, so always `null`. */
    connectableTs: number | null;
    /** When the actor went to sleep. Not recorded by this version, so always `null`. */
    sleepTs: number | null;
    /** When the actor will next try to allocate. Not recorded by this version, so always `null`. */
    rescheduleTs: number | null;
    /** When the actor was destroyed, in Unix milliseconds. Only set in listings that include destroyed actors; otherwise `null`. */
    destroyTs: number | null;
}
