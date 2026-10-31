import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const Node: core.serialization.ObjectSchema<serializers.Node.Raw, Rivet.Node> = core.serialization.object({
    key: core.serialization.string(),
    pool: core.serialization.string(),
    namespace: core.serialization.string(),
    datacenter: core.serialization.string(),
    createTs: core.serialization.property("create_ts", core.serialization.number()),
    lastPingTs: core.serialization.property("last_ping_ts", core.serialization.number()),
    connected: core.serialization.boolean(),
    stopping: core.serialization.boolean(),
});

export declare namespace Node {
    export interface Raw {
        key: string;
        pool: string;
        namespace: string;
        datacenter: string;
        create_ts: number;
        last_ping_ts: number;
        connected: boolean;
        stopping: boolean;
    }
}
