import * as Rivet from "../index";

export interface Node {
    /** Node key, unique within the pool. */
    key: string;
    /** Name of the node pool. */
    pool: string;
    /** Name of the namespace. */
    namespace: string;
    /** Datacenter name. */
    datacenter: string;
    /** When the node connected, in Unix milliseconds. */
    createTs: number;
    /** When the node last sent a heartbeat, in Unix milliseconds. */
    lastPingTs: number;
    /** Whether the node is connected. */
    connected: boolean;
    /** Whether the node is shutting down. */
    stopping: boolean;
}
