import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const ActorsRescheduleResponse: core.serialization.Schema<serializers.ActorsRescheduleResponse.Raw, Rivet.ActorsRescheduleResponse> = core.serialization.object({});

export declare namespace ActorsRescheduleResponse {
    export type Raw = Record<string, never>;
}
