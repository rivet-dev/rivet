import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { RivetId } from "./RivetId";

export const Actor: core.serialization.ObjectSchema<serializers.Actor.Raw, Rivet.Actor> = core.serialization.object({
    actorId: core.serialization.property("actor_id", RivetId),
    name: core.serialization.string(),
    key: core.serialization.string(),
    namespace: core.serialization.string(),
    pool: core.serialization.string(),
    createTs: core.serialization.property("create_ts", core.serialization.number()),
    startTs: core.serialization.property("start_ts", core.serialization.number() .nullable()),
    pendingAllocationTs: core.serialization.property("pending_allocation_ts", core.serialization.number() .nullable()),
    connectableTs: core.serialization.property("connectable_ts", core.serialization.number() .nullable()),
    sleepTs: core.serialization.property("sleep_ts", core.serialization.number() .nullable()),
    rescheduleTs: core.serialization.property("reschedule_ts", core.serialization.number() .nullable()),
    destroyTs: core.serialization.property("destroy_ts", core.serialization.number() .nullable()),
});

export declare namespace Actor {
    export interface Raw {
        actor_id: RivetId.Raw;
        name: string;
        key: string;
        namespace: string;
        pool: string;
        create_ts: number;
        start_ts: number | null;
        pending_allocation_ts: number | null;
        connectable_ts: number | null;
        sleep_ts: number | null;
        reschedule_ts: number | null;
        destroy_ts: number | null;
    }
}
