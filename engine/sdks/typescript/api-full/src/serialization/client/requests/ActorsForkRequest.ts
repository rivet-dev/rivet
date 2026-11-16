import * as serializers from "../../index";
import * as Rivet from "../../../api/index";
import * as core from "../../../core";

export const ActorsForkRequest: core.serialization.Schema<
    serializers.ActorsForkRequest.Raw,
    Omit<Rivet.ActorsForkRequest, "namespace">
> = core.serialization.object({
    key: core.serialization.string().optional(),
    input: core.serialization.string().optional(),
});

export declare namespace ActorsForkRequest {
    export interface Raw {
        key?: string | null;
        input?: string | null;
    }
}
