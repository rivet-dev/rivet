import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const Namespace: core.serialization.ObjectSchema<serializers.Namespace.Raw, Rivet.Namespace> = core.serialization.object({
    name: core.serialization.string(),
    createTs: core.serialization.property("create_ts", core.serialization.number()),
});

export declare namespace Namespace {
    export interface Raw {
        name: string;
        create_ts: number;
    }
}
