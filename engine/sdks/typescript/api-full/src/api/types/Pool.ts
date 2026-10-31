import * as Rivet from "../index";

export interface Pool {
    /** Name of the node pool. */
    name: string;
    /** Drain nodes running an older version when a newer version connects. */
    drainOnVersionUpgrade: boolean;
    /** Seconds to wait before evicting actors from a draining node. */
    actorEvictionDelay: number;
    /** Seconds over which actors are evicted from a draining node. */
    actorEvictionPeriod: number;
    /** Rate at which actors are evicted from a draining node. Must be finite and greater than 0. */
    actorEvictionRate: number;
    /** Opaque JSON stored with the pool, or `null` when unset. */
    metadata: unknown;
}
