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
    /**
     * When the actor first started. Not recorded, so always `null`.
     * @deprecated
     */
    startTs: number | null;
    /**
     * When the actor started waiting for a node. Not recorded, so always `null`. An actor with no node to start on sleeps and retries instead of waiting.
     * @deprecated
     */
    pendingAllocationTs: number | null;
    /** When a running actor became connectable, in Unix milliseconds. `null` while the actor is not running, and for an actor that has not started since the engine began recording it. */
    connectableTs: number | null;
    /** When a sleeping actor went to sleep, in Unix milliseconds. `null` while the actor is not asleep, and for an actor that has not slept since the engine began recording it. */
    sleepTs: number | null;
    /**
     * When the actor will next try to allocate. Not recorded, so always `null`.
     * @deprecated
     */
    rescheduleTs: number | null;
    /** When the actor was destroyed, in Unix milliseconds. Only set in listings that include destroyed actors; otherwise `null`. */
    destroyTs: number | null;
    /** ID of the source actor this actor's database was forked from, or `null` for an actor that was not made by a fork. Servers released before forks omit it. */
    forkedFrom?: Rivet.RivetId | null;
}
