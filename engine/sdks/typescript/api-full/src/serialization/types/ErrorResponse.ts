import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const ErrorResponse: core.serialization.ObjectSchema<serializers.ErrorResponse.Raw, Rivet.ErrorResponse> = core.serialization.object({
    group: core.serialization.string(),
    code: core.serialization.string(),
    message: core.serialization.string(),
    metadata: core.serialization.unknown().optional(),
});

export declare namespace ErrorResponse {
    export interface Raw {
        group: string;
        code: string;
        message: string;
        metadata?: unknown;
    }
}
