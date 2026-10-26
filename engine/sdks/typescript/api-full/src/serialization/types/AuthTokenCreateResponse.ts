import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const AuthTokenCreateResponse: core.serialization.ObjectSchema<serializers.AuthTokenCreateResponse.Raw, Rivet.AuthTokenCreateResponse> = core.serialization.object({
    token: core.serialization.string(),
    issuedTs: core.serialization.property("issued_ts", core.serialization.number()),
    expiresTs: core.serialization.property("expires_ts", core.serialization.number()),
});

export declare namespace AuthTokenCreateResponse {
    export interface Raw {
        token: string;
        issued_ts: number;
        expires_ts: number;
    }
}
