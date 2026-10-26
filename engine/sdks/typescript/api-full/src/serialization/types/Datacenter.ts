import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const Datacenter: core.serialization.ObjectSchema<serializers.Datacenter.Raw, Rivet.Datacenter> = core.serialization.object({
    name: core.serialization.string(),
});

export declare namespace Datacenter {
    export interface Raw {
        name: string;
    }
}
