import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { Datacenter } from "./Datacenter";

export const DatacentersListResponse: core.serialization.ObjectSchema<serializers.DatacentersListResponse.Raw, Rivet.DatacentersListResponse> = core.serialization.object({
    datacenters: core.serialization.list(Datacenter),
});

export declare namespace DatacentersListResponse {
    export interface Raw {
        datacenters: (Datacenter.Raw)[];
    }
}
