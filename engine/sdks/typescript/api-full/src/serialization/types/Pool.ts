import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const Pool: core.serialization.ObjectSchema<serializers.Pool.Raw, Rivet.Pool> = core.serialization.object({
    name: core.serialization.string(),
    drainOnVersionUpgrade: core.serialization.property("drain_on_version_upgrade", core.serialization.boolean()),
    actorEvictionDelay: core.serialization.property("actor_eviction_delay", core.serialization.number()),
    actorEvictionPeriod: core.serialization.property("actor_eviction_period", core.serialization.number()),
    actorEvictionRate: core.serialization.property("actor_eviction_rate", core.serialization.number()),
    metadata: core.serialization.unknown(),
}) as unknown as core.serialization.ObjectSchema<serializers.Pool.Raw, Rivet.Pool>;

export declare namespace Pool {
    export interface Raw {
        name: string;
        drain_on_version_upgrade: boolean;
        actor_eviction_delay: number;
        actor_eviction_period: number;
        actor_eviction_rate: number;
        metadata: unknown;
    }
}
