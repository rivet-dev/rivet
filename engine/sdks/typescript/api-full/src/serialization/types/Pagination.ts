import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const Pagination: core.serialization.ObjectSchema<serializers.Pagination.Raw, Rivet.Pagination> = core.serialization.object({
    cursor: core.serialization.string() .nullable(),
});

export declare namespace Pagination {
    export interface Raw {
        cursor: string | null;
    }
}
